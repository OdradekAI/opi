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
    stream_execution = {
        "stream_imports_execution": bool(
            matching_edges([src / "stream.rs"], r"crate::execution::")
        ),
        "execution_imports_stream": bool(
            matching_edges([src / "execution.rs"], r"crate::stream::")
        ),
    }
    models_lines = len((src / "models.rs").read_text(encoding="utf-8").splitlines())
    models_owner_modules = matching_edges(
        [src / "models.rs"], r"struct [A-Za-z_]+Owner\s*\{"
    )
    models_prepared_execution_owner = matching_edges(
        [src / "models.rs"], r"struct PreparedProviderCall\s*\{"
    )

    return {
        "api_provider_edges": api_provider,
        "provider_contract_api_edges": provider_contract_api,
        "profile_native_state": profile_native,
        "provider_to_provider_edges": provider_to_provider,
        "models_protocol_branches": models_protocol,
        "models_owner_modules": models_owner_modules,
        "models_prepared_execution_owner": models_prepared_execution_owner,
        "stream_execution_edges": stream_execution,
        "models_lines": models_lines,
        "counts": {
            "api_provider_edges": len(api_provider),
            "provider_contract_api_edges": len(provider_contract_api),
            "profile_native_state": len(profile_native),
            "provider_to_provider_edges": len(provider_to_provider),
            "models_protocol_branches": len(models_protocol),
            "models_owner_modules": len(models_owner_modules),
            "models_prepared_execution_owner": len(models_prepared_execution_owner),
        },
    }


def violations(report: dict[str, object]) -> list[str]:
    counts = report["counts"]
    result = []
    if counts["api_provider_edges"]:
        result.append("api adapters import concrete provider profiles")
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
