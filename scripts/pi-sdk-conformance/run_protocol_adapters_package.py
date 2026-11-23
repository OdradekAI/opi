"""Consume provider protocol adapters from an external caller."""

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

# The protocol adapter profile owns provider wire code and its protocol-only
# dependency closure, while the bundled catalog and provider collection remain
# separate builtins concerns.
def check_graph(metadata: dict, package: Path) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    sdk_features = set(nodes[sdk[0]["id"]]["features"])
    # `protocol-adapters` is the reviewed compatibility group and expands to
    # every provider protocol owner. Keep this exact union aligned with the
    # Cargo feature graph so the external package proof checks the real closure.
    expected_features = {
        "cloud-auth", "http-transport", "protocol-adapters",
        "provider-anthropic", "provider-azure-openai", "provider-bedrock",
        "provider-gemini", "provider-mistral", "provider-openai-chat",
        "provider-openai-codex-responses", "provider-openai-responses",
        "provider-openai-responses-shared", "provider-openrouter",
        "provider-openrouter-usage", "provider-pi-messages",
        "provider-protocol-common", "provider-vertex", "tls-backend",
        "websocket-compression",
    }
    if sdk_features != expected_features:
        raise ValueError(f"unexpected protocol feature union: {sorted(sdk_features)}")
    names = {entry["name"] for entry in packages}
    forbidden = {"opi-catalog"}.intersection(names)
    if forbidden:
        raise ValueError(f"protocol consumer contains forbidden catalog owner: {sorted(forbidden)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("protocol consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    required = {
        "reqwest", "rustls", "rustls-native-certs", "aws-lc-rs",
        "hmac", "aws-smithy-query", "aws-smithy-xml", "xmlparser",
        "p256", "sha1", "libz-rs-sys", "flate2",
    }
    present = required.intersection(names)
    if present != required:
        raise ValueError(f"protocol consumer lost required adapter owner: {sorted(required - present)}")
    return {
        "features": sorted(sdk_features),
        "catalog_absent": True,
        "protocol_present": sorted(present),
        "packages": sorted(entry["name"] + "@" + entry["version"] for entry in packages),
    }


def check_canonical_shared_layout(package: Path) -> dict:
    """Ensure the archive carries the shared decoder only at its canonical path."""
    root = package / "src" / "text_decode.rs"
    canonical = package / "src" / "api" / "shared" / "text_decode.rs"
    if root.exists():
        raise ValueError("archive still contains the deleted root text_decode module")
    if not canonical.exists():
        raise ValueError("archive is missing api/shared/text_decode.rs")
    return {
        "root_text_decode_absent": True,
        "canonical_text_decode_present": True,
    }


def check_canonical_endpoint_layout(package: Path) -> dict:
    """Ensure the archive carries URL joining only at its canonical path."""
    root = package / "src" / "endpoint.rs"
    canonical = package / "src" / "api" / "shared" / "endpoint.rs"
    if root.exists():
        raise ValueError("archive still contains the deleted root endpoint module")
    if not canonical.exists():
        raise ValueError("archive is missing api/shared/endpoint.rs")
    return {
        "root_endpoint_absent": True,
        "canonical_endpoint_present": True,
    }


def check_canonical_operation_contract_layout(package: Path) -> dict:
    """Ensure operation contracts are not retained in legacy owner modules."""
    deferred_root = package / "src" / "deferred.rs"
    contract = package / "src" / "api" / "contract.rs"
    models = package / "src" / "models.rs"
    if deferred_root.exists():
        raise ValueError("archive still contains the deleted root deferred module")
    if not contract.exists():
        raise ValueError("archive is missing api/contract.rs")
    if "pub trait ChatApi" in models.read_text(encoding="utf-8"):
        raise ValueError("archive still owns ChatApi in models.rs")
    return {
        "root_deferred_absent": True,
        "canonical_operation_contract_present": True,
        "models_chat_contract_absent": True,
    }


def check_openai_profile_layout(package: Path) -> dict:
    """Ensure OpenAI profiles do not retain API-native option state."""
    profile = package / "src" / "providers" / "openai.rs"
    source = profile.read_text(encoding="utf-8")
    if "native_options" in source:
        raise ValueError("OpenAI provider profile still stores native options")
    if "api::openai_completions::OpenAiCompletionsOptions" in source:
        raise ValueError("OpenAI provider profile still imports wire options")
    responses_profile = package / "src" / "providers" / "openai_responses.rs"
    responses_source = responses_profile.read_text(encoding="utf-8")
    if "native_options" in responses_source:
        raise ValueError("OpenAI Responses provider profile still stores native options")
    if "api::openai_responses::OpenAiResponsesOptions" in responses_source:
        raise ValueError("OpenAI Responses provider profile still imports wire options")
    codex_profile = package / "src" / "providers" / "openai_codex.rs"
    codex_source = codex_profile.read_text(encoding="utf-8")
    if "native_options" in codex_source:
        raise ValueError("OpenAI Codex provider profile still stores native options")
    if "crate::api" in codex_source or "api::" in codex_source:
        raise ValueError("OpenAI Codex provider profile still depends on the API layer")
    anthropic_profile = package / "src" / "providers" / "anthropic.rs"
    anthropic_source = anthropic_profile.read_text(encoding="utf-8")
    if "native_options" in anthropic_source:
        raise ValueError("Anthropic provider profile still stores native options")
    if "crate::api" in anthropic_source or "api::" in anthropic_source:
        raise ValueError("Anthropic provider profile still depends on the API layer")
    azure_profile = package / "src" / "providers" / "azure_openai.rs"
    azure_source = azure_profile.read_text(encoding="utf-8")
    if "native_options" in azure_source or "azure_native" in azure_source:
        raise ValueError("Azure OpenAI provider profile still stores native options")
    if "crate::api" in azure_source or "api::" in azure_source:
        raise ValueError("Azure OpenAI provider profile still depends on the API layer")
    return {
        "openai_profile_has_no_native_options": True,
        "openai_responses_profile_has_no_native_options": True,
        "openai_codex_profile_has_no_native_options": True,
        "openai_codex_profile_has_no_api_dependency": True,
        "anthropic_profile_has_no_native_options": True,
        "anthropic_profile_has_no_api_dependency": True,
        "azure_openai_profile_has_no_native_options": True,
        "azure_openai_profile_has_no_api_dependency": True,
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
        "family": "protocol-adapters-package",
        "scope": "external consumer of the protocol-adapters feature from the actual opi-ai archive",
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
        evidence["canonical_shared_layout"] = check_canonical_shared_layout(package)
        evidence["canonical_endpoint_layout"] = check_canonical_endpoint_layout(package)
        evidence["canonical_operation_contract_layout"] = check_canonical_operation_contract_layout(package)
        evidence["openai_profile_layout"] = check_openai_profile_layout(package)
        consumer = scratch / "consumer"
        (consumer / "src").mkdir(parents=True)
        manifest = (
            '[package]\nname = "opi-protocol-adapters-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\n[workspace]\n[dependencies]\n'
            f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
            'features = ["protocol-adapters"] }\n'
        )
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        (consumer / "src/main.rs").write_text(
            """use std::any::type_name;

use opi_ai::api::cloudflare_workers_ai_system_one::CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID;
use opi_ai::api::llama_cpp_classify::LLAMA_CPP_CLASSIFY_API_ID;
use opi_ai::api::openai_decisions::OPENAI_DECISIONS_API_ID;

fn main() {
    assert_eq!(OPENAI_DECISIONS_API_ID, "openai-decisions");
    assert_eq!(CLOUDFLARE_WORKERS_AI_SYSTEM_ONE_API_ID, "cloudflare-workers-ai-system-one");
    assert_eq!(LLAMA_CPP_CLASSIFY_API_ID, "llama-cpp-classify");
    let _ = opi_ai::api::anthropic::parse_sse_events("");
    let _ = opi_ai::api::openai_completions::parse_sse_events("");
    let _ = opi_ai::api::shared::headers::merge_provider_header_sources(&[]);
    let _ = opi_ai::api::shared::env::provider_env_value("OPI_EXTERNAL_CONSUMER_ENV", None);
    let _ = type_name::<dyn opi_ai::api::contract::ChatApi<()>>();
    let _ = type_name::<dyn opi_ai::api::contract::DeferredApi<()>>();
    let _ = type_name::<opi_ai::api::contract::DeferredFetchCall>();
    let _ = type_name::<opi_ai::types::deferred::DeferredHandle>();
    let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesOptions>(); let _ = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);
    let _ = type_name::<opi_ai::providers::mapped::ApiMappedProvider>();
    let _ = opi_ai::providers::openrouter::model_catalog();
    let _ = opi_ai::providers::mistral::mistral_provider(None);
    let _ = opi_ai::providers::pi_messages::PiMessagesProvider::new(Some("http://127.0.0.1:1".into()));
    let _ = opi_ai::providers::bedrock::is_standard_bedrock_endpoint("https://bedrock-runtime.us-east-1.amazonaws.com");
    let _ = std::any::type_name::<opi_ai::providers::bedrock::BedrockProvider>();
    let _ = std::any::type_name::<opi_ai::api::bedrock::BedrockOptions>();
    let _ = opi_ai::providers::vertex::VertexProvider::express(None);
    let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexOptions>();
    let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexChatApi>();
    let _ = opi_ai::providers::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test"));
    assert!(type_name::<opi_ai::auth::OAuthCredential>().contains("OAuthCredential"));
    assert!(type_name::<opi_ai::auth::store_oauth::StoreOAuthResolver>().contains("StoreOAuthResolver"));
    let nonce = opi_ai::ws::fresh_nonce();
    assert_eq!(nonce.len(), 16);
    println!("protocol-adapters=ok");
}
""",
            encoding="utf-8",
            newline="\n",
        )
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        metadata = json.loads(run(
            "protocol-adapters-metadata",
            ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"],
            consumer,
        ))
        evidence["graph"] = check_graph(metadata, package)
        run("protocol-adapters-build", ["cargo", "build", "--offline", "--locked"], consumer)
        output_text = run("protocol-adapters-run", ["cargo", "run", "--offline", "--locked"], consumer)
        if "protocol-adapters=ok" not in output_text:
            raise ValueError("external consumer did not observe the protocol adapter contract")
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
