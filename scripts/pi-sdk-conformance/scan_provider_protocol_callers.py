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
    "provider-openai-chat": ["openai_chat"],
    "provider-openai-codex-responses": ["openai_codex_responses", "codex_resources"],
    "provider-openai-responses": ["openai_responses", "openai_responses_shared"],
    "provider-openrouter": ["openrouter"],
    "provider-pi-messages": ["pi_messages"],
    "provider-vertex": ["vertex"],
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
        patterns = [
            re.compile(rf"(?:crate|opi_ai)::(?:{'|'.join(re.escape(module) for module in modules)})(?:::|\b)")
        ]
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
        "provider_collection": ["crates/opi-ai/src/provider_collection.rs"],
        "shared_openai_chat": ["crates/opi-ai/src/openai_chat.rs", "crates/opi-ai/src/mistral.rs", "crates/opi-ai/src/openrouter.rs", "crates/opi-ai/src/azure_openai.rs"],
        "shared_gemini": ["crates/opi-ai/src/gemini.rs", "crates/opi-ai/src/vertex.rs"],
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
