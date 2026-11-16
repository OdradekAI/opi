"""Verify the two remaining shared-owner capability V pair profiles."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tomllib

import package_slice
import run_early_package_proof as shared


REPO = Path(__file__).resolve().parents[2]

CLOUD_PACKAGES = {
    "hmac", "aws-smithy-query", "aws-smithy-xml", "xmlparser", "p256", "sha1",
}
TRANSPORT_PACKAGES = {"reqwest", "rustls", "rustls-native-certs", "aws-lc-rs"}
WS_PACKAGES = {"libz-rs-sys", "flate2"}

PAIRS = {
    "cloud-auth+catalog-bundled": {
        "features": ("cloud-auth", "catalog-bundled"),
        "tests": ("cloud_auth_profile", "catalog_artifact"),
        "required_packages": TRANSPORT_PACKAGES | CLOUD_PACKAGES,
        "forbidden_packages": WS_PACKAGES,
        "manifest_extra": "secrecy = \"0.10.3\"\n",
        "source": """use secrecy::{ExposeSecret, SecretString};

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
    let catalog = opi_ai::catalog::Catalog::builtin().expect("embedded catalog loads");
    assert_eq!(catalog.provenance().counts.chat, 1608);
    println!("cloud-auth+catalog-bundled=ok");
}
""",
    },
    "websocket-compression+oauth-store": {
        "features": ("websocket-compression", "oauth-store"),
        "tests": ("websocket_compression_profile", "oauth_store_profile"),
        "required_packages": TRANSPORT_PACKAGES | WS_PACKAGES,
        "forbidden_packages": CLOUD_PACKAGES,
        "manifest_extra": "",
        "source": """use std::sync::Arc;

use opi_ai::auth::oauth::anthropic::AnthropicOAuthProvider;
use opi_ai::auth::store_oauth::StoreOAuthResolver;
use opi_ai::auth::AuthResolver;
use opi_ai::mutation::InMemoryCredentialStore;

fn main() {
    let flow = Arc::new(AnthropicOAuthProvider::production());
    let store = Arc::new(InMemoryCredentialStore::new());
    let _: Arc<dyn AuthResolver> = Arc::new(StoreOAuthResolver::new(store, "anthropic", flow));
    let verifier = opi_ai::auth::oauth::generate_code_verifier();
    assert_eq!(verifier.len(), 43);
    let nonce = opi_ai::ws::fresh_nonce();
    assert_eq!(nonce.len(), 16);
    assert!(!opi_ai::ws::accept_token(&nonce).is_empty());
    println!("websocket-compression+oauth-store=ok");
}
""",
    },
}


def feature_closure(manifest: dict, selected: set[str]) -> set[str]:
    features = manifest.get("features", {})
    result: set[str] = set()
    pending = list(selected)
    while pending:
        feature = pending.pop()
        if feature in result:
            continue
        result.add(feature)
        for dependency in features.get(feature, []):
            if dependency.startswith("dep:") or "/" in dependency:
                continue
            pending.append(dependency)
    return result


def run_step(evidence: dict, name: str, command: list[str], cwd: Path,
             env: dict[str, str]) -> tuple[int, str]:
    started = time.monotonic()
    result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True,
                            encoding="utf-8", errors="replace", timeout=3600)
    paths = {}
    for stream in ("stdout", "stderr"):
        path = Path(evidence["scratch"]) / f"{name}.{stream}"
        path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
        paths[stream] = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    evidence["steps"].append({"name": name, "command": command, "exit": result.returncode,
                               "elapsed_seconds": round(time.monotonic() - started, 2), **paths})
    return result.returncode, result.stdout


def native_profiles(target: Path, scratch: Path, selected: list[tuple[str, dict]]) -> dict:
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env["CARGO_TARGET_DIR"] = str(target)
    manifest = tomllib.loads((REPO / "crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))
    receipt = {"target": str(target), "profiles": [], "steps": [], "verdict": "PASS",
               "scratch": str(scratch)}
    for index, (name, definition) in enumerate(selected, start=1):
        prefix = f"native-{index:02d}"
        features = definition["features"]
        feature_arg = ",".join(features)
        check = run_step(receipt, f"{prefix}-check", ["cargo", "check", "-p", "opi-ai",
                                                       "--locked", "--offline", "--no-default-features",
                                                       "--features", feature_arg], REPO, env)[0]
        test_command = ["cargo", "test", "-p", "opi-ai", "--locked", "--offline",
                        "--no-default-features", "--features", feature_arg]
        for test_name in definition["tests"]:
            test_command += ["--test", test_name]
        test_command += ["--", "--nocapture"]
        test = run_step(receipt, f"{prefix}-profile", test_command, REPO, env)[0]
        expected = feature_closure(manifest, set(features))
        verdict = "PASS" if check == 0 and test == 0 else "FAIL"
        receipt["profiles"].append({"id": name, "features": list(features),
                                    "expected_feature_closure": sorted(expected),
                                    "check_exit": check, "profile_exit": test,
                                    "profile_targets": list(definition["tests"]),
                                    "verdict": verdict})
        if verdict != "PASS":
            receipt["verdict"] = "FAIL"
    return receipt


def check_package_graph(metadata: dict, package: Path, name: str, definition: dict,
                        manifest: dict) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    actual = set(nodes[sdk[0]["id"]]["features"])
    expected = feature_closure(manifest, set(definition["features"]))
    if actual != expected:
        raise ValueError(f"unexpected feature closure: expected {sorted(expected)}, got {sorted(actual)}")
    names = {entry["name"] for entry in packages}
    missing = definition["required_packages"] - names
    if missing:
        raise ValueError(f"{name} consumer lost required owner packages: {sorted(missing)}")
    forbidden = definition["forbidden_packages"].intersection(names)
    if forbidden:
        raise ValueError(f"{name} consumer contains forbidden owner packages: {sorted(forbidden)}")
    if "opi-catalog" in names:
        raise ValueError(f"{name} consumer unexpectedly resolves opi-catalog")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    return {"features": sorted(actual), "required_present": sorted(definition["required_packages"]),
            "forbidden_absent": sorted(definition["forbidden_packages"]),
            "packages": sorted(f"{p['name']}@{p['version']}" for p in packages)}


def package_profiles(target: Path, scratch: Path, selected: list[tuple[str, dict]]) -> dict:
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    env.setdefault("CARGO_PROFILE_TEST_DEBUG", "0")
    env["CARGO_TARGET_DIR"] = str(target)
    manifest = tomllib.loads((REPO / "crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))
    receipt = {"target": str(target), "profiles": [], "steps": [], "verdict": "PASS",
               "scratch": str(scratch)}
    candidate = None

    def run(name: str, command: list[str], cwd: Path) -> str:
        nonlocal candidate
        if candidate is not None:
            command = candidate.command(command)
        code, stdout = run_step(receipt, name, command, cwd, env)
        if code:
            raise ValueError(f"{name} failed with exit {code}")
        if candidate is not None and command[0] == "cargo" and "metadata" in command:
            candidate.verify_graph(json.loads(stdout))
        return stdout

    package, package_evidence, candidate = shared.package_sdk(run, scratch, target)
    receipt["package"] = package_evidence
    for index, (name, definition) in enumerate(selected, start=1):
        consumer = scratch / f"consumer-{index:02d}"
        (consumer / "src").mkdir(parents=True)
        literals = ", ".join(json.dumps(feature) for feature in definition["features"])
        manifest_text = (
            f'[package]\nname = "opi-shared-owner-pair-{index:02d}"\nversion = "0.0.0"\n'
            f'edition = "2024"\n[workspace]\n[dependencies]\n'
            f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
            f'features = [{literals}] }}\n{definition["manifest_extra"]}'
        )
        (consumer / "Cargo.toml").write_text(manifest_text, encoding="utf-8", newline="\n")
        (consumer / "src/main.rs").write_text(definition["source"], encoding="utf-8", newline="\n")
        prefix = f"package-{index:02d}"
        graph = None
        output = ""
        output_code = 1
        try:
            run(f"{prefix}-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
            metadata = json.loads(run(f"{prefix}-metadata", ["cargo", "metadata", "--format-version", "1",
                                                               "--offline", "--locked"], consumer))
            graph = check_package_graph(metadata, package, name, definition, manifest)
            run(f"{prefix}-build", ["cargo", "build", "--offline", "--locked"], consumer)
            output = run(f"{prefix}-run", ["cargo", "run", "--offline", "--locked"], consumer)
            output_code = 0
        except (OSError, ValueError, subprocess.SubprocessError) as exc:
            output = str(exc)
        marker = f"{name}=ok"
        verdict = "PASS" if output_code == 0 and marker in output else "FAIL"
        receipt["profiles"].append({"id": name, "features": list(definition["features"]),
                                    "graph": graph, "consumer": str(consumer), "marker": marker,
                                    "verdict": verdict})
        if verdict != "PASS":
            receipt["verdict"] = "FAIL"
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("native", "package", "both"), default="both")
    parser.add_argument("--only", action="append", help="verify only this pair id; repeat for multiple pairs")
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    args = parser.parse_args()
    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    output = Path(args.evidence_output).expanduser().resolve()
    if output.exists():
        parser.error("evidence already exists; select a new path")
    selected_names = args.only or list(PAIRS)
    unknown = set(selected_names) - set(PAIRS)
    if unknown:
        parser.error(f"unknown pair id; choose from {sorted(PAIRS)}")
    selected = [(name, PAIRS[name]) for name in selected_names]
    shared.validate_paths(REPO, scratch, target)
    scratch.mkdir(parents=True)
    evidence = {
        "family": "shared-owner-pair-capability-v",
        "scope": "cloud-auth+catalog-bundled and websocket-compression+oauth-store shared-owner pairs",
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "pair_count": len(selected), "profiles_expected": len(selected),
        "selected_pairs": selected_names, "native": None, "package": None, "verdict": "PASS",
        "frozen_pi": {"status": "retained", "reference": "frozen Pi 1.5.0 paired differential receipts retained unchanged"},
        "macos": {"status": "BLOCKED", "reason": "no user-provided macOS runner or SSH host"},
        "scratch": str(scratch), "steps": [],
    }
    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        env = package_slice.controlled_environment(dict(os.environ), target)
        run_step(evidence, "rustc-version", ["rustc", "-vV"], REPO, env)
        run_step(evidence, "cargo-version", ["cargo", "-vV"], REPO, env)
        code, _ = run_step(evidence, "lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO, env)
        if code:
            raise ValueError(f"lease-start failed with exit {code}")
        leased = True
        if args.mode in ("native", "both"):
            evidence["native"] = native_profiles(target, scratch, selected)
            if evidence["native"]["verdict"] != "PASS":
                evidence["verdict"] = "FAIL"
        if args.mode in ("package", "both"):
            evidence["package"] = package_profiles(target, scratch, selected)
            if evidence["package"]["verdict"] != "PASS":
                evidence["verdict"] = "FAIL"
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        evidence["verdict"] = "FAIL"
        evidence["error"] = str(exc)
    finally:
        if leased:
            try:
                env = package_slice.controlled_environment(dict(os.environ), target)
                run_step(evidence, "lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO, env)
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
