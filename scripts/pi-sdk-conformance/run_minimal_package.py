"""Execute core + scripted Faux from actual archives and reject backend leakage."""
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

HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parents[1]
TESTS = {"core_profile": 3, "streaming_json": 12,
         "incremental_argument_preview": 3, "final_numeric_decoding": 2}
FORBIDDEN = {"reqwest", "rustls", "rustls-native-certs", "aws-lc-rs", "aws-lc-sys",
             "aws-smithy-query", "aws-smithy-xml", "p256", "hmac", "sha1", "xmlparser",
             "libz-rs-sys", "flate2", "opi-agent", "opi-coding-agent", "opi-tui", "opi-protocol"}


def check_graph(metadata, package, tracing=False):
    sdk = [p for p in metadata["packages"] if p["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("minimum consumer must resolve the extracted SDK archive")
    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    features = set(nodes[sdk[0]["id"]]["features"])
    if features != ({"telemetry-tracing"} if tracing else set()):
        raise ValueError(f"unexpected minimum feature union: {features}")
    leaked = FORBIDDEN.intersection(p["name"] for p in metadata["packages"])
    if leaked:
        raise ValueError(f"minimum profile contains forbidden backends: {sorted(leaked)}")
    local_id = shared.candidate_local_id(metadata, package)
    if local_id is None or ("tracing" in nodes[local_id]["features"]) != tracing:
        raise ValueError("companion telemetry tracing feature was not exercised")
    for p in metadata["packages"]:
        if p.get("source") is None and p["id"] not in {sdk[0]["id"], local_id, metadata["resolve"]["root"]}:
            raise ValueError(f"unexpected local dependency: {p['name']}")
    return {"features": sorted(features), "telemetry_features": nodes[local_id]["features"],
            "forbidden_absent": sorted(FORBIDDEN),
            "packages": sorted(p["name"] + "@" + p["version"] for p in metadata["packages"])}


def check_catalog_absence(stdout):
    errors = [row["message"] for line in stdout.splitlines()
              if (row := json.loads(line)).get("reason") == "compiler-message"
              and row["message"]["level"] == "error"]
    if (len(errors) != 1 or (errors[0].get("code") or {}).get("code") != "E0599"
            or "`builtin`" not in errors[0]["message"] or "`Catalog`" not in errors[0]["message"]):
        raise ValueError("negative catalog probe failed for a different reason")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    args = parser.parse_args()
    scratch, target, output = (Path(x).resolve() for x in (args.scratch, args.target_dir, args.evidence_output))
    if output.exists():
        parser.error("evidence already exists; select a new path")
    shared.validate_paths(REPO, scratch, target)
    env = controlled_environment(dict(os.environ), target)
    configs = check_cargo_configs(env, [REPO, scratch / "consumer", scratch / "package"])
    env.setdefault("CARGO_BUILD_JOBS", "4")
    scratch.mkdir(parents=True)
    evidence = {"family": "minimal-package", "verdict": "FAIL", "scratch": str(scratch),
                "cargo_configs": configs, "target": str(target), "steps": [],
                "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    candidate = None
    def run(name, command, cwd, expected=0):
        if candidate is not None:
            command = candidate.command(command)
        start = time.monotonic()
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=3600)
        row = {"name": name, "command": command, "exit": result.returncode,
               "elapsed_seconds": round(time.monotonic()-start, 2)}
        for stream in ("stdout", "stderr"):
            path = scratch / f"{name}.{stream}"
            path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
            row[stream + "_sha256"] = hashlib.sha256(path.read_bytes()).hexdigest()
        evidence["steps"].append(row)
        if result.returncode != expected:
            raise ValueError(f"{name} failed: expected {expected}, got {result.returncode}; see scratch logs")
        return result.stdout, result.stderr
    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        evidence["rustc"] = run("rustc", ["rustc", "-vV"], REPO)[0]
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        package, evidence["package"], candidate = shared.package_sdk(lambda n,c,d: run(n,c,d)[0], scratch, target)
        consumer = scratch / "consumer"
        (consumer / "tests").mkdir(parents=True)
        manifest = ('[package]\nname="opi-minimum-consumer"\nversion="0.0.0"\nedition="2024"\n'
                    '[workspace]\n[dependencies]\nopi-ai={path=' + json.dumps(package.as_posix()) + ',default-features=false}\n'
                    'serde_json="=1.0.149"\ntokio={version="=1.52.3",features=["macros","rt-multi-thread","sync","time"]}\n'
                    'futures-util="=0.3.32"\ntokio-util="=0.7.18"\ntracing={version="=0.1.44",optional=true}\n'
                    '[features]\ntelemetry-tracing=["opi-ai/telemetry-tracing","dep:tracing"]\n')
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        evidence["tests"] = {}
        for name, count in TESTS.items():
            source = package / "tests" / (name + ".rs")
            shutil.copyfile(source, consumer / "tests" / source.name)
            evidence["tests"][name] = {"sha256": hashlib.sha256(source.read_bytes()).hexdigest(), "expected_passed": count}
        fixture = package / "tests/data/incremental-argument-preview.json"
        (consumer / "tests/data").mkdir()
        shutil.copyfile(fixture, consumer / "tests/data" / fixture.name)
        evidence["fixture_sha256"] = hashlib.sha256(fixture.read_bytes()).hexdigest()
        run("lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        evidence["profiles"] = []
        for tracing in (False, True):
            name = "core-tracing" if tracing else "core"
            flags = ["--features", "telemetry-tracing"] if tracing else []
            metadata = json.loads(run(name + "-metadata", ["cargo", "metadata", "--offline", "--locked", "--format-version", "1", *flags], consumer)[0])
            candidate.verify_graph(metadata)
            evidence["profiles"].append({"name": name, **check_graph(metadata, package, tracing)})
            run(name + "-clippy", ["cargo", "clippy", "--offline", "--locked", "--tests", *flags, "--", "-D", "warnings"], consumer)
            for test, count in TESTS.items():
                count += int(tracing and test == "core_profile")
                stdout, _ = run(name + "-" + test, ["cargo", "test", "--offline", "--locked", "--test", test, *flags, "--", "--nocapture"], consumer)
                if not re.search(rf"test result: ok\. {count} passed; 0 failed; 0 ignored;", stdout):
                    raise ValueError(f"{name}/{test}: incomplete test execution")
        env["RUSTDOCFLAGS"] = "-D warnings"
        run("core-doc", ["cargo", "doc", "--offline", "--locked", "-p", "opi-ai", "--no-deps"], consumer)
        # Compile-negative evidence pins the absence of embedded catalog APIs.
        (consumer / "src").mkdir()
        (consumer / "src/main.rs").write_text('fn main() { let _ = opi_ai::catalog::Catalog::builtin(); }\n', encoding="utf-8")
        stdout, _ = run("builtin-catalog-absent", ["cargo", "check", "--offline", "--locked", "--bin", "opi-minimum-consumer", "--message-format=json"], consumer, expected=101)
        check_catalog_absence(stdout)
        evidence["verdict"] = "PASS"
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        evidence["error"] = str(exc)
        print(exc, file=sys.stderr)
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.SubprocessError) as exc:
                evidence["verdict"], evidence["lease_error"] = "FAIL", str(exc)
        encoded = json.dumps(evidence, indent=2) + "\n"
        (scratch / "evidence.json").write_text(encoded, encoding="utf-8", newline="\n")
        with output.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(encoded)
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
