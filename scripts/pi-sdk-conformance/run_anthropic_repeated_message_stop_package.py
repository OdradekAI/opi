"""Consume the current SDK archive through the Anthropic message_stop tail test."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import time

import run_early_package_proof as shared
from package_slice import check_cargo_configs, controlled_environment
from run_readme_examples import check_opi_feature_profile

REPO = Path(__file__).resolve().parents[2]


def check_execution(stdout: str) -> None:
    match = re.search(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored; 0 measured;", stdout)
    if not match or int(match.group(1)) < 1:
        raise ValueError("the Anthropic message_stop tail contract test must execute")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("scratch", "target-dir", "evidence-output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    scratch, target, output = (p.resolve() for p in (args.scratch, args.target_dir, args.evidence_output))
    if output.exists():
        parser.error(f"refusing to overwrite evidence: {output}")
    shared.validate_paths(REPO, scratch, target)
    env = controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    configs = check_cargo_configs(env, [REPO, scratch / "consumer", scratch / "package"])
    scratch.mkdir(parents=True)
    evidence = {
        "verdict": "FAIL",
        "scope": "Anthropic message_stop repeated usage and tail observer ordering consumed from actual SDK archives in builtins/no-tracing and defaults profiles",
        "steps": [], "profiles": [], "cargo_configs": configs,
    }
    candidate = None

    def run(name: str, command: list[str], cwd: Path) -> str:
        if candidate is not None:
            command = candidate.command(command)
        started = time.monotonic()
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, timeout=3600)
        row = {"name": name, "command": command, "exit": result.returncode, "seconds": round(time.monotonic() - started, 2)}
        for label in ("stdout", "stderr"):
            path = scratch / f"{name}.{label}"
            path.write_bytes(getattr(result, label))
            row[label] = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        evidence["steps"].append(row)
        if result.returncode:
            raise ValueError(f"{name} failed; see {scratch}")
        return result.stdout.decode("utf-8")

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        package, evidence["package"], candidate = shared.package_sdk(run, scratch, target)
        consumer = scratch / "consumer"
        (consumer / "tests" / "data").mkdir(parents=True)
        files = ["tests/anthropic_repeated_message_stop.rs", "tests/data/anthropic-repeated-message-stop.json"]
        evidence["executed_sources"] = {}
        for name in files:
            archive = package / name
            source = REPO / "crates/opi-ai" / name
            if archive.read_bytes() != source.read_bytes():
                raise ValueError(f"archive source mismatch: {name}")
            shutil.copyfile(archive, consumer / name)
            evidence["executed_sources"][name] = hashlib.sha256(archive.read_bytes()).hexdigest()
        manifest = (
            '[package]\nname="opi-anthropic-repeated-message-stop-consumer"\nversion="0.0.0"\nedition="2024"\n[workspace]\n'
            '[dependencies]\n'
            f'opi-ai={{path={json.dumps(package.as_posix())},default-features=false,features=["builtins"]}}\n'
            'serde_json="=1.0.149"\n'
            'futures-util="=0.3.32"\n'
            'tokio-util="=0.7.18"\n'
            'tokio={version="=1.52.3",features=["macros","rt-multi-thread"]}\n'
            '[features]\ncaller-defaults=["opi-ai/default"]\n'
        )
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        run("lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        for defaults in (False, True):
            name = "defaults" if defaults else "builtins-no-tracing"
            flags = ["--features", "caller-defaults"] if defaults else []
            metadata = json.loads(run(name + "-metadata", ["cargo", "metadata", "--offline", "--locked", "--format-version", "1", *flags], consumer))
            telemetry_id = candidate.verify_graph(metadata)
            telemetry_features = next(node["features"] for node in metadata["resolve"]["nodes"] if node["id"] == telemetry_id)
            evidence["profiles"].append({"name": name, **check_opi_feature_profile(metadata, package, defaults), "telemetry_features": sorted(telemetry_features)})
            run(name + "-clippy", ["cargo", "clippy", "--offline", "--locked", "--tests", *flags, "--", "-D", "warnings"], consumer)
            stdout = run(name + "-tests", ["cargo", "test", "--offline", "--locked", "--test", "anthropic_repeated_message_stop", *flags, "--", "--nocapture"], consumer)
            check_execution(stdout)
        evidence["verdict"] = "PASS"
    finally:
        if leased:
            run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
        evidence["runner_sha256"] = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"verdict": evidence["verdict"], "profiles": len(evidence["profiles"])}))


if __name__ == "__main__":
    main()
