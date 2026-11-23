"""Verify provider-owner pair and high-order feature unions natively and from a package."""

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
PACKAGE_RUNNER = Path(__file__).with_name("run_provider_protocol_package.py")

PROVIDERS = {
    "provider-anthropic": 'let _ = opi_ai::api::anthropic::parse_sse_events(""); let _ = std::any::type_name::<opi_ai::providers::anthropic::AnthropicProvider>();',
    "provider-azure-openai": 'let _ = opi_ai::providers::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test")); let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIOptions>();',
    "provider-bedrock": 'assert!(opi_ai::providers::bedrock::is_standard_bedrock_endpoint("https://bedrock-runtime.us-east-1.amazonaws.com")); let _ = std::any::type_name::<opi_ai::providers::bedrock::BedrockProvider>(); let _ = std::any::type_name::<opi_ai::api::bedrock::BedrockOptions>();',
    "provider-gemini": "let _ = std::any::type_name::<opi_ai::api::gemini::GoogleGenerativeAiOptions>(); let _ = std::any::type_name::<opi_ai::providers::gemini::GeminiProvider>();",
    "provider-mistral": "let _ = opi_ai::providers::mistral::mistral_provider(None); let _ = std::any::type_name::<opi_ai::api::mistral::MistralOptions>(); let _ = std::any::type_name::<opi_ai::providers::mistral::MistralConversationsProvider>();",
    "provider-openai-chat": 'let _ = opi_ai::api::openai_completions::parse_sse_events(""); let _ = std::any::type_name::<opi_ai::api::openai_completions::OpenAiCompletionsOptions>(); let _ = std::any::type_name::<opi_ai::providers::openai::OpenAiChatProvider>();',
    "provider-openai-codex-responses": "let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexOptions>(); let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexChatApi>(); let _ = std::any::type_name::<opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider>();",
    "provider-openai-responses": "let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesOptions>(); let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesChatApi>(); let _ = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);",
    "provider-openrouter": "let _ = opi_ai::providers::openrouter::model_catalog();",
    "provider-pi-messages": 'let _ = std::any::type_name::<opi_ai::api::pi_messages::PiMessagesOptions>(); let _ = std::any::type_name::<opi_ai::providers::pi_messages::PiMessagesProvider>();',
    "provider-vertex": "let _ = opi_ai::providers::vertex::VertexProvider::express(None); let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexOptions>(); let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexChatApi>();",
}

HIGH_ORDER = {
    "all-provider-owners": tuple(PROVIDERS),
    "all-provider-owners-plus-support": tuple(PROVIDERS)
    + ("catalog-bundled", "oauth-store", "telemetry-tracing"),
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


def profile_name(features: tuple[str, ...]) -> str:
    return "+".join(features)


def provider_pairs() -> list[tuple[str, ...]]:
    names = list(PROVIDERS)
    return [(names[index], names[other])
            for index in range(len(names))
            for other in range(index + 1, len(names))]


def profiles(only: set[str] | None = None) -> list[tuple[str, tuple[str, ...], str]]:
    result = [(profile_name(pair), pair, "pair") for pair in provider_pairs()]
    result.extend((name, tuple(features), "higher-order") for name, features in HIGH_ORDER.items())
    return [profile for profile in result if only is None or profile[0] in only]


def profile_targets(features: tuple[str, ...]) -> list[str]:
    return [f"provider_{name.removeprefix('provider-').replace('-', '_')}_profile"
            for name in features if name in PROVIDERS]


def run_step(evidence: dict, name: str, command: list[str], cwd: Path, env: dict[str, str]) -> tuple[int, str]:
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


def native_profiles(target: Path, scratch: Path, selected: list[tuple[str, tuple[str, ...], str]]) -> dict:
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env["CARGO_TARGET_DIR"] = str(target)
    manifest = tomllib.loads((REPO / "crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))
    receipt = {"target": str(target), "profiles": [], "steps": [], "verdict": "PASS", "scratch": str(scratch)}
    for index, (name, features, kind) in enumerate(selected, start=1):
        prefix = f"native-{index:02d}"
        feature_arg = ",".join(features)
        check = run_step(receipt, f"{prefix}-check", ["cargo", "check", "-p", "opi-ai",
                                                       "--locked", "--offline", "--no-default-features",
                                                       "--features", feature_arg], REPO, env)[0]
        targets = profile_targets(features)
        test_command = ["cargo", "test", "-p", "opi-ai", "--locked", "--offline",
                        "--no-default-features", "--features", feature_arg]
        for target_name in targets:
            test_command += ["--test", target_name]
        test_command += ["--", "--nocapture"]
        test = run_step(receipt, f"{prefix}-profile", test_command, REPO, env)[0]
        expected = feature_closure(manifest, set(features))
        verdict = "PASS" if check == 0 and test == 0 else "FAIL"
        receipt["profiles"].append({"id": name, "kind": kind, "features": list(features),
                                    "expected_feature_closure": sorted(expected),
                                    "check_exit": check, "profile_exit": test,
                                    "profile_targets": targets, "verdict": verdict})
        if verdict != "PASS":
            receipt["verdict"] = "FAIL"
    return receipt


def check_package_graph(metadata: dict, package: Path, selected: tuple[str, ...], manifest: dict) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    actual = set(nodes[sdk[0]["id"]]["features"])
    expected = feature_closure(manifest, set(selected))
    if actual != expected:
        raise ValueError(f"unexpected feature closure: expected {sorted(expected)}, got {sorted(actual)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    names = {entry["name"] for entry in packages}
    if "catalog-bundled" not in selected and "opi-catalog" in names:
        raise ValueError("provider-only consumer unexpectedly contains catalog package")
    return {"features": sorted(actual), "packages": sorted(f"{p['name']}@{p['version']}" for p in packages),
            "catalog_present": "opi-catalog" in names}


def package_profiles(target: Path, scratch: Path, selected: list[tuple[str, tuple[str, ...], str]]) -> dict:
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    # The package matrix is a feature-closure proof, so retain incremental
    # reuse but omit debug symbols from each external consumer binary. This
    # keeps the persistent shared target bounded without changing codegen or
    # the selected feature graph.
    env.setdefault("CARGO_PROFILE_DEV_DEBUG", "0")
    env.setdefault("CARGO_PROFILE_TEST_DEBUG", "0")
    env["CARGO_TARGET_DIR"] = str(target)
    manifest = tomllib.loads((REPO / "crates/opi-ai/Cargo.toml").read_text(encoding="utf-8"))
    receipt = {"target": str(target), "profiles": [], "steps": [], "verdict": "PASS", "scratch": str(scratch)}
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
    for index, (name, features, kind) in enumerate(selected, start=1):
        consumer = scratch / f"consumer-{index:02d}"
        (consumer / "src").mkdir(parents=True)
        feature_literals = ", ".join(json.dumps(feature) for feature in features)
        (consumer / "Cargo.toml").write_text(
            f'[package]\nname = "opi-provider-matrix-{index:02d}"\nversion = "0.0.0"\nedition = "2024"\n'
            f'[workspace]\n[dependencies]\nopi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, features = [{feature_literals}] }}\n',
            encoding="utf-8", newline="\n")
        calls = [PROVIDERS[feature] for feature in features if feature in PROVIDERS]
        if "catalog-bundled" in features:
            calls.append('assert!(opi_ai::catalog::Catalog::builtin().is_ok());')
        body = "\n    ".join(calls)
        (consumer / "src/main.rs").write_text(
            f"fn main() {{\n    {body}\n    println!(\"{name}=ok\");\n}}\n",
            encoding="utf-8", newline="\n")
        prefix = f"package-{index:02d}"
        graph = None
        try:
            run(f"{prefix}-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
            metadata_output = run(f"{prefix}-metadata", ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"], consumer)
            graph = check_package_graph(json.loads(metadata_output), package, features, manifest)
            run(f"{prefix}-build", ["cargo", "build", "--offline", "--locked"], consumer)
            output = run(f"{prefix}-run", ["cargo", "run", "--offline", "--locked"], consumer)
            output_code = 0
        except (OSError, ValueError, subprocess.SubprocessError) as exc:
            output = str(exc)
            output_code = 1
        expected_marker = f"{name}=ok"
        verdict = "PASS" if output_code == 0 and expected_marker in output else "FAIL"
        receipt["profiles"].append({"id": name, "kind": kind, "features": list(features), "graph": graph,
                                    "consumer": str(consumer), "marker": expected_marker, "verdict": verdict})
        if verdict != "PASS":
            receipt["verdict"] = "FAIL"
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("native", "package", "both"), default="both")
    parser.add_argument("--only", action="append", help="verify only this profile id; repeat for multiple ids")
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    args = parser.parse_args()
    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    output = Path(args.evidence_output).expanduser().resolve()
    if output.exists():
        parser.error("evidence already exists; select a new path")
    shared.validate_paths(REPO, scratch, target)
    scratch.mkdir(parents=True)
    selected = profiles(set(args.only) if args.only else None)
    if args.only and len(selected) != len(args.only):
        known = {profile[0] for profile in profiles()}
        parser.error(f"unknown profile id; choose from {sorted(known)}")
    evidence = {
        "family": "provider-protocol-capability-matrix",
        "scope": "all unordered provider-owner pairs plus two high-order provider-owner unions",
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "provider_count": len(PROVIDERS), "pair_count": len(provider_pairs()),
        "high_order_count": len(HIGH_ORDER), "profiles_expected": len(selected),
        "native": None, "package": None, "verdict": "PASS",
        "frozen_pi": {
            "status": "retained",
            "reference": "phase5/phase8 frozen Pi 1.5.0 paired differential receipts; no source behavior change in this feature-union validation",
        },
        "macos": {"status": "BLOCKED", "reason": "no user-provided macOS runner or SSH host"},
        "scratch": str(scratch),
        "steps": [],
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
