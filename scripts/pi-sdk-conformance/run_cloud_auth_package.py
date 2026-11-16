"""Consume protocol-independent cloud authentication from an external caller."""

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

REPO = Path(__file__).resolve().parents[2]


def check_graph(metadata: dict, package: Path) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    sdk_features = set(nodes[sdk[0]["id"]]["features"])
    expected_features = {"cloud-auth", "http-transport", "tls-backend"}
    if sdk_features != expected_features:
        raise ValueError(f"unexpected cloud-auth feature union: {sorted(sdk_features)}")
    names = {entry["name"] for entry in packages}
    forbidden = {"opi-catalog", "protocol-adapters", "libz-rs-sys", "flate2"}.intersection(names)
    if forbidden:
        raise ValueError(f"cloud-auth consumer contains forbidden provider/compression owner: {sorted(forbidden)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("cloud-auth consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    required = {
        "reqwest", "rustls", "rustls-native-certs", "aws-lc-rs",
        "hmac", "aws-smithy-query", "aws-smithy-xml", "xmlparser",
        "p256", "sha1",
    }
    present = required.intersection(names)
    if present != required:
        raise ValueError(f"cloud-auth consumer lost required owner packages: {sorted(required - present)}")
    return {
        "features": sorted(sdk_features),
        "provider_protocol_absent": True,
        "compression_absent": True,
        "cloud_auth_present": sorted(present),
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
        "family": "cloud-auth-package",
        "scope": "external consumer of the cloud-auth feature from the actual opi-ai archive",
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
            command, cwd=cwd, env=env, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=3600,
        )
        paths = {}
        for stream in ("stdout", "stderr"):
            path = scratch / f"{name}.{stream}"
            path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
            paths[stream] = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        evidence["steps"].append({"name": name, "command": command,
                                  "exit": result.returncode,
                                  "elapsed_seconds": round(time.monotonic() - started, 2), **paths})
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
            '[package]\nname = "opi-cloud-auth-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\n[workspace]\n[dependencies]\n'
            'secrecy = "0.10.3"\n'
            f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
            'features = ["cloud-auth"] }\n'
        )
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        (consumer / "src/main.rs").write_text(
            """use secrecy::{ExposeSecret, SecretString};

fn main() {
    let endpoint = opi_ai::auth::aws::endpoint::sts_endpoint_url("us-east-1", None);
    assert_eq!(endpoint, "https://sts.us-east-1.amazonaws.com");
    let credentials = opi_ai::auth::AwsSigV4Credentials::new(
        SecretString::from("AKIDEXAMPLE"),
        SecretString::from("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY"),
        "us-east-1",
    );
    let signed = opi_ai::auth::aws::sigv4::sign_request(
        "POST", "/", "Action=GetCallerIdentity&Version=2011-06-15",
        &[("host", "sts.us-east-1.amazonaws.com")], b"", &credentials,
        "sts", "20150830", "20150830T123600Z",
    );
    assert!(signed.authorization.expose_secret().starts_with("AWS4-HMAC-SHA256"));
    assert_eq!(opi_ai::auth::adc::native::CANONICAL_TOKEN_ENDPOINT,
               "https://oauth2.googleapis.com/token");
    println!("cloud-auth=ok");
}
""",
            encoding="utf-8", newline="\n",
        )
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        metadata = json.loads(run(
            "cloud-auth-metadata", ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"], consumer
        ))
        evidence["graph"] = check_graph(metadata, package)
        run("cloud-auth-build", ["cargo", "build", "--offline", "--locked"], consumer)
        output_text = run("cloud-auth-run", ["cargo", "run", "--offline", "--locked"], consumer)
        if "cloud-auth=ok" not in output_text:
            raise ValueError("external consumer did not observe the cloud-auth contract")
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
