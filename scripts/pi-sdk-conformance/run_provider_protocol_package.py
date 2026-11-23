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
        "source": 'let _ = opi_ai::api::anthropic::parse_sse_events(""); let _ = std::any::type_name::<opi_ai::api::anthropic::AnthropicMessagesWireAdapter>(); let _ = std::any::type_name::<opi_ai::providers::anthropic::AnthropicProvider>();',
    },
    "provider-azure-openai": {
        "features": {"http-transport", "provider-azure-openai", "provider-openai-chat", "provider-openai-responses", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": 'let _ = opi_ai::providers::azure_openai::resolve_azure_base_url(None, None, Some("https://example.test")); let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIOptions>(); let _ = std::any::type_name::<opi_ai::api::azure_openai::AzureOpenAIWireAdapter>(); let _ = std::any::type_name::<opi_ai::providers::azure_openai::AzureOpenAIProvider>();',
    },
    "provider-bedrock": {
        "features": {"cloud-auth", "http-transport", "provider-bedrock", "provider-protocol-common", "tls-backend"},
        "source": """use opi_ai::providers::bedrock::{BedrockProvider, is_standard_bedrock_endpoint, model_catalog};
    use opi_ai::provider::Provider;
    assert!(is_standard_bedrock_endpoint("https://bedrock-runtime.us-east-1.amazonaws.com"));
    let models = model_catalog();
    let provider = BedrockProvider::new(None, std::sync::Arc::new(opi_ai::http::HttpClient::new()));
    assert_eq!(provider.models(), models);
    assert_eq!(models.len(), 197);
    let terra = models.iter().find(|model| model.id == "global.openai.gpt-6.1-terra").unwrap();
    assert!(terra.compat.as_bedrock_converse_stream().unwrap().supports_strict_tools);
    for id in [
        "anthropic.claude-sonnet-4-20250514-v2:0",
        "anthropic.claude-opus-4-20250514-v1:0",
        "anthropic.claude-haiku-4-5-20250514-v1:0",
    ] {
        assert!(models.iter().all(|model| model.id != id));
    }
    let _ = std::any::type_name::<opi_ai::api::bedrock::BedrockOptions>();""",
    },
    "provider-gemini": {
        "features": {"http-transport", "provider-gemini", "provider-protocol-common", "tls-backend"},
        "source": """use opi_ai::provider::Provider;
    use opi_ai::providers::gemini::{GeminiProvider, google_provider, model_catalog};
    let _ = std::any::type_name::<opi_ai::api::gemini::GoogleGenerativeAiOptions>();
    let models = model_catalog();
    assert_eq!(models.len(), 23);
    assert_eq!(GeminiProvider::new(None).models(), models);
    let google = google_provider(None, std::sync::Arc::new(opi_ai::http::HttpClient::new()));
    assert_eq!(google.models(), models);
    assert_eq!(google.id(), "google");
    for id in ["gemini-pro-latest", "gemini-robotics-er-2-preview"] {
        let model = models.iter().find(|model| model.id == id).unwrap();
        assert_eq!(model.wire_api, opi_ai::WireApi::GoogleGenerativeAi);
        assert!(model.capabilities.supports_thinking);
    }
    for id in ["gemini-2.0-flash", "gemini-3.1-flash-lite-image", "gemini-3.1-flash-live-preview"] {
        assert!(models.iter().all(|model| model.id != id));
    }""",
    },
    "provider-mistral": {
        "features": {"http-transport", "provider-mistral", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": "let _ = opi_ai::providers::mistral::mistral_provider(None); let _ = std::any::type_name::<opi_ai::api::mistral::MistralOptions>(); let _ = std::any::type_name::<opi_ai::providers::mistral::MistralConversationsProvider>();",
    },
    "provider-openai-chat": {
        "features": {"http-transport", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": 'let _ = opi_ai::api::openai_completions::parse_sse_events(""); let _ = std::any::type_name::<opi_ai::api::openai_completions::OpenAiCompletionsOptions>(); let _ = std::any::type_name::<opi_ai::providers::openai::OpenAiChatProvider>();',
    },
    "provider-openai-codex-responses": {
        "features": {"http-transport", "provider-openai-codex-responses", "provider-openai-responses-shared", "provider-protocol-common", "tls-backend", "websocket-compression"},
        "source": "let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexOptions>(); let _ = std::any::type_name::<opi_ai::api::openai_codex_responses::CodexChatApi>(); let _ = std::any::type_name::<opi_ai::providers::openai_codex::OpenAiCodexResponsesProvider>();",
    },
    "provider-openai-responses": {
        "features": {"http-transport", "provider-openai-responses", "provider-openai-responses-shared", "provider-protocol-common", "tls-backend"},
        "source": "let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesOptions>(); let _ = std::any::type_name::<opi_ai::api::openai_responses::OpenAiResponsesChatApi>(); let _ = opi_ai::providers::openai_responses::OpenAiResponsesProvider::new(None);",
    },
    "provider-openrouter": {
        "features": {"http-transport", "provider-openai-chat", "provider-openai-responses-shared", "provider-openrouter", "provider-openrouter-usage", "provider-protocol-common", "tls-backend"},
        "source": """use opi_ai::providers::openrouter::{
        OpenRouterCostDetails, OpenRouterUsageAccounting, apply_openrouter_reported_cost,
        model_catalog,
    };
    let models = model_catalog();
    let ids: Vec<_> = models.iter().map(|model| model.id.as_str()).collect();
    assert_eq!(ids, ["openai/gpt-4o", "openai/gpt-4o-mini", "google/gemini-2.5-pro"]);
    for model in &models {
        assert!(model.capabilities.context_window >= model.capabilities.max_output_tokens);
        assert!(model.base_url.is_none() && model.pricing.is_some());
    }
    let mut usage = opi_ai::types::usage::Usage::default();
    let charge = OpenRouterUsageAccounting {
        cost: Some(0.25),
        is_byok: Some(true),
        cost_details: Some(OpenRouterCostDetails {
            upstream_inference_cost: Some(0.5),
            upstream_inference_prompt_cost: None,
            upstream_inference_completions_cost: None,
            server_tool_cost: None,
        }),
    };
    apply_openrouter_reported_cost(&mut usage, &charge);
    assert_eq!(usage.cost.total, 0.75);
    assert!(usage.cost.total_from_provider);""",
    },
    "provider-pi-messages": {
        "features": {"http-transport", "provider-pi-messages", "provider-protocol-common", "tls-backend"},
        "source": "let _ = std::any::type_name::<opi_ai::api::pi_messages::PiMessagesOptions>(); let _ = std::any::type_name::<opi_ai::providers::pi_messages::PiMessagesProvider>();",
    },
    "provider-vertex": {
        "features": {"cloud-auth", "http-transport", "provider-gemini", "provider-protocol-common", "provider-vertex", "tls-backend"},
        "source": """use opi_ai::provider::Provider;
    use opi_ai::providers::vertex::{VertexProvider, google_vertex_provider, model_catalog};
    let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexOptions>();
    let _ = std::any::type_name::<opi_ai::api::vertex::GoogleVertexChatApi>();
    let models = model_catalog();
    assert_eq!(models.len(), 13);
    assert_eq!(VertexProvider::express(None).models(), models);
    let provider = google_vertex_provider(
        Some(("fixture-project".into(), "eu".into())),
        None,
        std::sync::Arc::new(opi_ai::http::HttpClient::new()),
    );
    assert_eq!(provider.models(), models);
    assert_eq!(provider.id(), "google-vertex");
    assert!(models.iter().all(|model| {
        model.wire_api == opi_ai::WireApi::GoogleVertex && model.id != "gemini-3.1-flash-lite-image"
    }));""",
    },
}

CLOUD_PACKAGES = {"hmac", "aws-smithy-query", "aws-smithy-xml", "xmlparser", "p256", "sha1"}
WS_PACKAGES = {"libz-rs-sys", "flate2"}


def check_canonical_operation_contract_layout(package: Path) -> dict:
    """Ensure the packaged operation contracts have one canonical owner."""
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
    api_dir = package / "src" / "api"
    if (api_dir / "openai.rs").exists():
        raise ValueError("archive still contains the ambiguous api/openai.rs module")
    if not (api_dir / "openai_completions.rs").is_file():
        raise ValueError("archive is missing api/openai_completions.rs")
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
        "openai_completions_wire_is_canonical": True,
        "openai_profile_has_no_native_options": True,
        "openai_responses_profile_has_no_native_options": True,
        "openai_codex_profile_has_no_native_options": True,
        "openai_codex_profile_has_no_api_dependency": True,
        "anthropic_profile_has_no_native_options": True,
        "anthropic_profile_has_no_api_dependency": True,
        "azure_openai_profile_has_no_native_options": True,
        "azure_openai_profile_has_no_api_dependency": True,
    }


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
        evidence["canonical_operation_contract_layout"] = check_canonical_operation_contract_layout(package)
        evidence["openai_profile_layout"] = check_openai_profile_layout(package)
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
