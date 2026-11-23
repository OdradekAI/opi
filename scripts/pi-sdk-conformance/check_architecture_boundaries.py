"""Report and enforce the opi-ai ownership boundaries.

The report intentionally measures dependency edges and owner violations. It
does not infer completion from directory names or file counts.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys

from lower_layer_edges import lower_layer_report
from profile_api_edges import profile_api_edges


REPO = Path(__file__).resolve().parents[2]
SRC = REPO / "crates" / "opi-ai" / "src"

PROFILE_FILES = (
    "bedrock.rs",
    "gemini.rs",
    "mistral.rs",
    "pi_messages.rs",
    "vertex.rs",
)


def code_lines(path: Path) -> list[tuple[int, str]]:
    """Return source lines while ignoring line and rustdoc comments."""

    lines = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.lstrip()
        if stripped.startswith("//"):
            continue
        lines.append((number, line))
    return lines


def matching_edges(paths: list[Path], pattern: str) -> list[dict[str, object]]:
    expression = re.compile(pattern)
    matches = []
    for path in paths:
        for number, line in code_lines(path):
            if expression.search(line):
                matches.append(
                    {
                        "file": path.relative_to(REPO).as_posix(),
                        "line": number,
                        "text": line.strip(),
                    }
                )
    return matches


def architecture_report(repo: Path = REPO) -> dict[str, object]:
    src = repo / "crates" / "opi-ai" / "src"
    api_files = sorted((src / "api").rglob("*.rs"))
    profile_dir = src / "providers"
    profile_files = [profile_dir / name for name in PROFILE_FILES]

    api_provider = matching_edges(api_files, r"crate::providers::")
    api_route_context = matching_edges(api_files, r"crate::provider::routes::")
    profile_api = profile_api_edges(repo)
    lower = lower_layer_report(repo)
    api_contract_provider = matching_edges(
        [src / "api" / "contract.rs"], r"crate::provider(?!::operation\b)"
    )
    provider_contract_api = matching_edges([src / "provider.rs"], r"crate::api(?:::|\b)")
    profile_native = matching_edges(
        profile_files,
        r"native_options|with_native_options|crate::api\.[A-Za-z_]+::[A-Za-z_]+Options",
    )
    provider_to_provider = matching_edges(
        [
            path
            for path in sorted(profile_dir.glob("*.rs"))
            if path.name not in {"collection.rs", "mapped.rs", "mod.rs"}
        ],
        r"crate::providers::",
    )
    models_protocol = matching_edges(
        [src / "models.rs"],
        r"WireApi::|supports_bedrock_adaptive_thinking|AnthropicMessages",
    )
    stream_execution_imports = [
        match
        for match in matching_edges([src / "stream.rs"], r"crate::execution::")
        if not match["text"].lstrip().startswith("pub use ")
    ]
    stream_execution_reexports = matching_edges(
        [src / "stream.rs"], r"pub use crate::execution::"
    )
    stream_execution = {
        "stream_imports_execution": bool(stream_execution_imports),
        "stream_reexports_execution": bool(stream_execution_reexports),
        "execution_imports_stream": bool(
            matching_edges([src / "execution.rs"], r"crate::stream::")
        ),
    }
    models_lines = len((src / "models.rs").read_text(encoding="utf-8").splitlines())
    models_module_paths = sorted((src / "models").glob("*.rs"))
    models_owner_paths = [src / "models.rs", *models_module_paths]
    models_owner_modules = matching_edges(
        models_owner_paths, r"struct [A-Za-z_]+Owner\s*\{"
    )
    models_owner_module_files = sorted(
        {
            match["file"]
            for match in matching_edges(models_module_paths, r"struct [A-Za-z_]+Owner\s*\{")
        }
    )
    models_owner_definitions_in_facade = matching_edges(
        [src / "models.rs"], r"struct [A-Za-z_]+Owner\s*\{"
    )
    models_prepared_execution_owner = matching_edges(
        models_owner_paths, r"struct PreparedProviderCall\s*\{"
    )
    models_prepared_execution_owner_files = sorted(
        {
            match["file"]
            for match in matching_edges(
                models_module_paths, r"struct PreparedProviderCall\s*\{"
            )
        }
    )
    registration_record_pattern = (
        r"struct (?:ImageApiRegistration|ClassifierApiRegistration|"
        r"DeferredApiRegistration|ChatApiRegistration|ChatApiEntry)\s*\{"
    )
    models_registration_records = matching_edges(
        models_owner_paths, registration_record_pattern
    )
    models_registration_record_files = sorted(
        {
            match["file"]
            for match in matching_edges(models_module_paths, registration_record_pattern)
        }
    )
    models_registration_records_in_facade = matching_edges(
        [src / "models.rs"], registration_record_pattern
    )
    chat_call_pattern = (
        r"(?:struct (?:FrozenChatBinding|ChatCall|ChatCallDriver|ChatCallSetup|"
        r"ResourceApiStream)|enum (?:FrozenRoute|AuthPlan|ChatCallState))\s*\{"
    )
    models_chat_call_orchestration = matching_edges(models_owner_paths, chat_call_pattern)
    models_chat_call_files = sorted(
        {
            match["file"]
            for match in matching_edges(models_module_paths, chat_call_pattern)
        }
    )
    models_chat_call_orchestration_in_facade = matching_edges(
        [src / "models.rs"], chat_call_pattern
    )

    return {
        "api_provider_edges": api_provider,
        "profile_api_edges": profile_api,
        "lower_layer_dependencies": lower,
        "api_route_context_edges": api_route_context,
        "api_contract_provider_edges": api_contract_provider,
        "provider_contract_api_edges": provider_contract_api,
        "profile_native_state": profile_native,
        "provider_to_provider_edges": provider_to_provider,
        "models_protocol_branches": models_protocol,
        "models_owner_modules": models_owner_modules,
        "models_owner_module_files": models_owner_module_files,
        "models_owner_definitions_in_facade": models_owner_definitions_in_facade,
        "models_prepared_execution_owner": models_prepared_execution_owner,
        "models_prepared_execution_owner_files": models_prepared_execution_owner_files,
        "models_registration_records": models_registration_records,
        "models_registration_record_files": models_registration_record_files,
        "models_registration_records_in_facade": models_registration_records_in_facade,
        "models_chat_call_orchestration": models_chat_call_orchestration,
        "models_chat_call_files": models_chat_call_files,
        "models_chat_call_orchestration_in_facade": models_chat_call_orchestration_in_facade,
        "stream_execution_edges": stream_execution,
        "models_lines": models_lines,
        "counts": {
            "api_provider_edges": len(api_provider),
            "profile_api_edges": len(profile_api),
            "unapproved_profile_api_edges": sum(edge["permission"] is None for edge in profile_api),
            "lower_layer_reviewed_contracts": len(lower["reviewed_contracts"]),
            "lower_layer_known_exceptions": len(lower["known_exceptions"]),
            "lower_layer_new_dependencies": len(lower["new_dependencies"]),
            "lower_layer_unused_baseline_sites": sum(
                edge["unused_sites"] for edge in lower["unused_baseline"]
            ),
            "lower_layer_excluded_test_sites": len(lower["excluded_tests"]),
            "provider_copilot_policy_sites": len(lower["provider_copilot_policy"]),
            "api_route_context_edges": len(api_route_context),
            "api_contract_provider_edges": len(api_contract_provider),
            "provider_contract_api_edges": len(provider_contract_api),
            "profile_native_state": len(profile_native),
            "provider_to_provider_edges": len(provider_to_provider),
            "models_protocol_branches": len(models_protocol),
            "models_owner_modules": len(models_owner_modules),
            "models_owner_module_files": len(models_owner_module_files),
            "models_owner_definitions_in_facade": len(models_owner_definitions_in_facade),
            "models_prepared_execution_owner": len(models_prepared_execution_owner),
            "models_prepared_execution_owner_files": len(models_prepared_execution_owner_files),
            "models_registration_records": len(models_registration_records),
            "models_registration_record_files": len(models_registration_record_files),
            "models_registration_records_in_facade": len(models_registration_records_in_facade),
            "models_chat_call_orchestration": len(models_chat_call_orchestration),
            "models_chat_call_files": len(models_chat_call_files),
            "models_chat_call_orchestration_in_facade": len(
                models_chat_call_orchestration_in_facade
            ),
        },
    }


def violations(report: dict[str, object]) -> list[str]:
    counts = report["counts"]
    result = []
    if counts["api_provider_edges"]:
        result.append("api adapters import concrete provider profiles")
    if counts["unapproved_profile_api_edges"]:
        result.append("provider profiles have unapproved API dependencies (see --json for sites)")
    if counts["lower_layer_new_dependencies"]:
        result.append("lower owners have new guarded dependencies (see --json for sites)")
    if counts["lower_layer_unused_baseline_sites"]:
        result.append("lower-layer baseline has unused entries (see --json for sites)")
    if counts["provider_copilot_policy_sites"]:
        result.append("lower provider owns Copilot header policy (see --json for sites)")
    if counts["api_contract_provider_edges"]:
        result.append("api contract imports provider implementation types directly")
    if counts["provider_contract_api_edges"]:
        result.append("provider.rs imports api contract code")
    if counts["profile_native_state"]:
        result.append("provider profiles retain API-native state")
    if counts["provider_to_provider_edges"]:
        result.append("provider profiles import other concrete provider profiles")
    if report["stream_execution_edges"]["stream_imports_execution"] and report["stream_execution_edges"]["execution_imports_stream"]:
        result.append("stream and execution own each other's implementation types")
    if counts["models_protocol_branches"]:
        result.append("Models retains protocol-specific branches")
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--enforce", action="store_true")
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()

    report = architecture_report()
    report["violations"] = violations(report)
    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(json.dumps({"counts": report["counts"], "violations": report["violations"]}, ensure_ascii=False, indent=2))
    return 1 if args.enforce and report["violations"] else 0


if __name__ == "__main__":
    sys.exit(main())
