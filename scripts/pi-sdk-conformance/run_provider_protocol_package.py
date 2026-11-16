"""Consume each provider protocol/source owner from an external caller."""

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

OWNERS = {
    "provider-anthropic": {
        "features": {"http-transport", "provider-anthropic", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": 'let _ = opi_ai::anthropic::parse_sse_events("");',
    },
    "provider-azure-openai": {
        "features": {"http-transport", "provider-azure-openai", "provider-openai-chat", "provider-openai-responses", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": 'let _ = opi_ai::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test"));',
    },
    "provider-bedrock": {
        "features": {"cloud-auth", "http-transport", "provider-bedrock", "provider-protocol-common", "tls-backend"},
        "source": 'assert!(opi_ai::bedrock::is_standard_bedrock_endpoint("https://bedrock-runtime.us-east-1.amazonaws.com"));',
    },
    "provider-gemini": {
        "features": {"http-transport", "provider-gemini", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::gemini::GeminiProvider::new(None);",
    },
    "provider-mistral": {
        "features": {"http-transport", "provider-mistral", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::mistral::mistral_provider(None);",
    },
    "provider-openai-chat": {
        "features": {"http-transport", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": 'let _ = opi_ai::openai_chat::parse_sse_events("");',
    },
    "provider-openai-codex-responses": {
        "features": {"http-transport", "provider-openai-codex-responses", "provider-openai-responses-shared", "provider-protocol-common", "tls-backend", "websocket-compression"},
        "source": "let _ = std::any::type_name::<opi_ai::openai_codex_responses::CodexOptions>();",
    },
    "provider-openai-responses": {
        "features": {"http-transport", "provider-openai-responses", "provider-openai-responses-shared", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::openai_responses::OpenAiResponsesProvider::new(None);",
    },
    "provider-openrouter": {
        "features": {"http-transport", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::openrouter::model_catalog();",
    },
    "provider-pi-messages": {
        "features": {"http-transport", "provider-pi-messages", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::pi_messages::PiMessagesProvider::new(None);",
    },
    "provider-vertex": {
        "features": {"cloud-auth", "http-transport", "provider-gemini", "provider-protocol-common", "provider-vertex", "tls-backend"},
        "source": "let _ = opi_ai::vertex::VertexProvider::express(None);",
    },
}

CLOUD_PACKAGES = {"hmac", "aws-smithy-query", "aws-smithy-xml", "xmlparser", "p256", "sha1"}
WS_PACKAGES = {"libz-rs-sys", "flate2"}


def check_graph(metadata: dict, package: Path, owner: str) -> dict:
    try:
        expected = OWNERS[owner]
    except KeyError as exc:
        raise ValueError(f"unknown provider owner: {owner}") from exc
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    sdk_features = set(nodes[sdk[0]["id"]]["features"])
    if sdk_features != expected["features"]:
        raise ValueError(f"unexpected {owner} feature union: {sorted(sdk_features)}")
    names = {entry["name"] for entry in packages}
    forbidden = {"opi-catalog"}.intersection(names)
    if owner not in {"provider-bedrock", "provider-vertex"}:
        forbidden |= CLOUD_PACKAGES.intersection(names)
    if owner != "provider-openai-codex-responses":
        forbidden |= WS_PACKAGES.intersection(names)
    if forbidden:
        raise ValueError(f"{owner} consumer contains forbidden owner packages: {sorted(forbidden)}")
    required = {"reqwest", "rustls", "rustls-native-certs", "aws-lc-rs"}
    if owner in {"provider-bedrock", "provider-vertex"}:
        required |= CLOUD_PACKAGES
    if owner == "provider-openai-codex-responses":
        required |= WS_PACKAGES
    missing = required - names
    if missing:
        raise ValueError(f"{owner} consumer lost required packages: {sorted(missing)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError(f"{owner} consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    return {
        "owner": owner,
        "features": sorted(sdk_features),
        "catalog_absent": True,
        "cloud_auth_present": sorted(CLOUD_PACKAGES.intersection(names)),
        "websocket_compression_present": sorted(WS_PACKAGES.intersection(names)),
        "packages": sorted(entry["name"] + "@" + entry["version"] for entry in packages),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    parser.add_argument("--owner", choices=sorted(OWNERS), help="rerun one owner against a fresh package")
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
            [REPO, scratch / "package"],
        )
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env["CARGO_TARGET_DIR"] = str(target)
    evidence = {
        "family": "provider-protocol-package",
        "scope": "external consumers of each provider protocol/source owner from the actual opi-ai archive",
        "target": str(target),
        "cargo_configs": configs,
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "owners": [],
        "verdict": "FAIL",
    }
    candidate = None

    def run(name: str, command: list[str], cwd: Path) -> str:
        nonlocal candidate
        if candidate is not None:
            command = candidate.command(command)
        started = time.monotonic()
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True,
                                encoding="utf-8", errors="replace", timeout=3600)
        paths = {}
        for stream in ("stdout", "stderr"):
            path = scratch / f"{name}.{stream}"
            path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
            paths[stream] = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
        evidence.setdefault("steps", []).append({"name": name, "command": command,
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
        selected = [(args.owner, OWNERS[args.owner])] if args.owner else list(OWNERS.items())
        for index, (owner, config) in enumerate(selected, start=1):
            consumer = scratch / owner
            (consumer / "src").mkdir(parents=True)
            manifest = (
                f'[package]\nname = "opi-{owner.replace("-", "-")}-consumer"\nversion = "0.0.0"\n'
                'edition = "2024"\n[workspace]\n[dependencies]\n'
                f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
                f'features = ["{owner}"] }}\n'
            )
            (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
            (consumer / "src/main.rs").write_text(
                f'fn main() {{\n    {config["source"]}\n    println!("{owner}=ok");\n}}\n',
                encoding="utf-8", newline="\n",
            )
            prefix = f"owner-{index:02d}-{owner}"
            run(f"{prefix}-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
            metadata = json.loads(run(
                f"{prefix}-metadata", ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"], consumer,
            ))
            graph = check_graph(metadata, package, owner)
            run(f"{prefix}-build", ["cargo", "build", "--offline", "--locked"], consumer)
            output_text = run(f"{prefix}-run", ["cargo", "run", "--offline", "--locked"], consumer)
            if f"{owner}=ok" not in output_text:
                raise ValueError(f"external consumer did not observe {owner} contract")
            evidence["owners"].append({
                "owner": owner,
                "graph": graph,
                "consumer_sources": {
                    "Cargo.toml": hashlib.sha256((consumer / "Cargo.toml").read_bytes()).hexdigest(),
                    "src/main.rs": hashlib.sha256((consumer / "src/main.rs").read_bytes()).hexdigest(),
                    "Cargo.lock": hashlib.sha256((consumer / "Cargo.lock").read_bytes()).hexdigest(),
                },
            })
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
