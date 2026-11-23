"""Inventory per-provider protocol/source callers and source ownership."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re

REPO = Path(__file__).resolve().parents[2]
SOURCE_ROOTS = [REPO / "crates", REPO / "scripts", REPO / "docs"]
SKIP_PARTS = {"target", ".git", "node_modules", "evidence"}

OWNERS = {
    "provider-anthropic": ["anthropic"],
    "provider-azure-openai": ["azure_openai"],
    "provider-bedrock": ["bedrock"],
    "provider-gemini": ["gemini"],
    "provider-mistral": ["mistral"],
    "provider-openai-chat": ["openai", "openai_completions"],
    "provider-openai-codex-responses": ["openai_codex_responses", "codex_resources"],
    "provider-openai-responses": ["openai_responses"],
    "provider-openrouter": ["openrouter"],
    "provider-pi-messages": ["pi_messages"],
    "provider-vertex": ["vertex"],
}

CANONICAL_SHARED = {
    "provider-openai-responses": ["api::shared::openai_responses"],
}


def files() -> list[Path]:
    found = []
    for root in SOURCE_ROOTS:
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if not path.is_file() or any(part in SKIP_PARTS for part in path.parts):
                continue
            if path.suffix.lower() not in {".rs", ".py", ".md", ".toml", ".json"}:
                continue
            found.append(path)
    return sorted(set(found))


def main() -> int:
    inventory = {
        "schemaVersion": 1,
        "scope": "repository caller inventory for per-provider protocol/source owners",
        "root": str(REPO),
        "files_scanned": 0,
        "owners": {},
        "shared_callers": {},
    }
    candidates = files()
    inventory["files_scanned"] = len(candidates)
    for owner, modules in OWNERS.items():
        module_pattern = "|".join(re.escape(module) for module in modules)
        patterns = [
            re.compile(rf"(?:crate|opi_ai)::(?:{module_pattern})(?:::|\b)"),
            re.compile(rf"(?:crate|opi_ai)::(?:api|providers)::(?:{module_pattern})(?:::|\b)"),
        ]
        shared_pattern = CANONICAL_SHARED.get(owner, [])
        if shared_pattern:
            shared_pattern = "|".join(re.escape(module) for module in shared_pattern)
            patterns.append(re.compile(rf"(?:crate|opi_ai)::(?:{shared_pattern})(?:::|\b)"))
        matches = []
        for path in candidates:
            text = path.read_text(encoding="utf-8", errors="replace")
            count = sum(len(pattern.findall(text)) for pattern in patterns)
            if count:
                matches.append({
                    "path": str(path.relative_to(REPO)).replace("\\", "/"),
                    "matches": count,
                    "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                })
        inventory["owners"][owner] = {
            "modules": modules,
            "files": len(matches),
            "matches": sum(entry["matches"] for entry in matches),
            "callers": matches,
            "interpretation": "direct module references are provider-owned; provider_collection and builtins profile registration remain compatibility-group callers",
        }
    shared = {
        "provider_collection": ["crates/opi-ai/src/providers/collection.rs"],
        "shared_openai_chat": ["crates/opi-ai/src/api/openai_completions.rs", "crates/opi-ai/src/providers/openai.rs", "crates/opi-ai/src/api/mistral.rs", "crates/opi-ai/src/providers/mistral.rs", "crates/opi-ai/src/providers/openrouter.rs", "crates/opi-ai/src/api/azure_openai.rs"],
        "shared_openai_responses": ["crates/opi-ai/src/api/shared/openai_responses.rs", "crates/opi-ai/src/api/openai_responses.rs", "crates/opi-ai/src/providers/openai_responses.rs", "crates/opi-ai/src/api/azure_openai.rs", "crates/opi-ai/src/api/openai_codex_responses.rs", "crates/opi-ai/src/providers/openai_codex.rs"],
        "shared_openrouter_usage": ["crates/opi-ai/src/api/shared/openrouter_usage.rs", "crates/opi-ai/src/api/anthropic.rs", "crates/opi-ai/src/api/openai_completions.rs", "crates/opi-ai/src/providers/openrouter.rs"],
        "bedrock_profile_and_wire": ["crates/opi-ai/src/providers/bedrock.rs", "crates/opi-ai/src/api/bedrock.rs", "crates/opi-ai/src/api/bedrock/event_stream.rs", "crates/opi-ai/src/providers/collection.rs"],
        "mapped_provider_composition": ["crates/opi-ai/src/providers/mapped.rs", "crates/opi-ai/src/providers/collection.rs", "crates/opi-coding-agent/src/provider_factory.rs"],
        "shared_gemini": ["crates/opi-ai/src/api/gemini.rs", "crates/opi-ai/src/providers/gemini.rs", "crates/opi-ai/src/api/vertex.rs", "crates/opi-ai/src/providers/vertex.rs"],
        "shared_resource_contract": ["crates/opi-ai/src/codex_resources.rs", "crates/opi-ai/src/provider.rs", "crates/opi-ai/src/models.rs"],
    }
    for label, paths in shared.items():
        entries = []
        for relative in paths:
            path = REPO / relative
            if path.is_file():
                entries.append({"path": relative, "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        inventory["shared_callers"][label] = entries
    print(json.dumps(inventory, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
