#!/usr/bin/env python3
"""Package the authentic Pi published model catalog as opi-ai offline resources.

Delivery-input packer for opi-implement task 20.32 (DEL-10). This tool never
generates model data: it verifies a downloaded artifact byte-for-byte against
the revision pinned by the local Pi 1.3.0 tree, validates it with the same
rules Pi's own ``model-data.ts`` / ``hydrate-model-catalog.ts`` enforce, and
stages-then-atomically-emits the offline bundle consumed by
``opi_ai::catalog``.

Usage:
    python3 scripts/pi-catalog/pack.py --artifact <downloaded.json>   # pack
    python3 scripts/pi-catalog/pack.py --check                        # verify emitted bundle

Exit code is non-zero on any validation failure; nothing is written unless
every check passes.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent
PI_TREE = REPO_ROOT / ".repo" / "pi-1.3.0"
PIN_FILE = PI_TREE / "nix" / "model-catalog.json"
AGGREGATOR = PI_TREE / "packages" / "ai" / "src" / "models.generated.ts"
PI_MODEL_DATA = PI_TREE / "packages" / "ai" / "scripts" / "model-data.ts"
PI_HYDRATE = PI_TREE / "packages" / "ai" / "scripts" / "hydrate-model-catalog.ts"
SOURCE_EVIDENCE = (
    REPO_ROOT / "docs" / "research" / "pi-ai-opi-ai-alignment" / "pi-1.3.0-source-evidence.json"
)

BUNDLE_SCHEMA_VERSION = 1
DATA_SCHEMA_VERSION = 6  # Pi MODEL_DATA_SCHEMA_VERSION for the hydrated layout
BUNDLE_DIR = REPO_ROOT / "crates" / "opi-ai" / "resources" / "catalog" / "builtin"
CATALOG_NAME = "catalog.json"
MANIFEST_NAME = "manifest.json"

# Mirrors MODEL_DATA_IMPORT_PATTERN from Pi's scripts/model-data.ts.
AGGREGATOR_IMPORT_PATTERN = re.compile(
    r'import \{ [A-Z][A-Z0-9_]*_CLASSIFIER_MODELS, [A-Z][A-Z0-9_]*_IMAGE_MODELS, '
    r'[A-Z][A-Z0-9_]*_MODELS \} from "\./providers/([^"/]+)\.models\.ts";'
)

OPERATIONS = ("chat", "image", "classifier")


class PackError(Exception):
    """One concrete packaging or validation failure."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def pinned_revision() -> str:
    pin = json.loads(PIN_FILE.read_text(encoding="utf-8"))
    revision = pin.get("revision", "")
    if not revision.startswith("sha256-") or len(revision) != len("sha256-") + 64:
        raise PackError(f"invalid pin revision in {PIN_FILE}: {revision!r}")
    return revision


def catalog_url(revision: str) -> str:
    return f"https://pi.dev/api/models/revisions/{revision}?types=chat,image,classifier"


def expected_providers() -> list[str]:
    aggregator = AGGREGATOR.read_text(encoding="utf-8")
    providers = sorted(set(AGGREGATOR_IMPORT_PATTERN.findall(aggregator)))
    if not providers:
        raise PackError(f"no provider imports found in {AGGREGATOR}")
    return providers


def is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def validate_entry(provider: str, entry: object) -> tuple[str, str, str]:
    """Validate one catalog entry; return its (type, id, api) identity."""
    label = f"{provider}/<unparsed>"
    if not isinstance(entry, dict):
        raise PackError(f"{label}: entry must be an object")
    model_type = entry.get("type")
    model_id = entry.get("id")
    api = entry.get("api")
    if not (isinstance(model_type, str) and model_type in OPERATIONS):
        raise PackError(f"{label}: type must be one of {OPERATIONS}, got {model_type!r}")
    if not (isinstance(model_id, str) and model_id):
        raise PackError(f"{label}: id must be a non-empty string")
    if not (isinstance(api, str) and api):
        raise PackError(f"{provider}/{model_type}:{model_id}: api must be a non-empty string")
    label = f"{provider}/{model_type}:{model_id}"

    if entry.get("provider") != provider:
        raise PackError(f"{label}: provider field {entry.get('provider')!r} != {provider!r}")
    name = entry.get("name")
    if not (isinstance(name, str) and name):
        raise PackError(f"{label}: missing non-empty name")
    if not isinstance(entry.get("baseUrl"), str):
        raise PackError(f"{label}: missing baseUrl string")

    inputs = entry.get("input")
    if not (
        isinstance(inputs, list)
        and inputs
        and all(item in ("text", "image") for item in inputs)
    ):
        raise PackError(f"{label}: invalid input modalities {inputs!r}")

    if model_type == "image":
        outputs = entry.get("output")
        if not (isinstance(outputs, list) and "image" in outputs):
            raise PackError(f"{label}: image entry needs image output modality")
    elif "output" in entry:
        raise PackError(f"{label}: {model_type} entry must not declare output modalities")

    if model_type in ("chat", "classifier"):
        context_window = entry.get("contextWindow")
        if not (is_number(context_window) and context_window > 0):
            raise PackError(f"{label}: invalid contextWindow {context_window!r}")
    if model_type == "chat":
        if not isinstance(entry.get("reasoning"), bool):
            raise PackError(f"{label}: missing reasoning boolean")
        max_tokens = entry.get("maxTokens")
        if not (is_number(max_tokens) and max_tokens > 0):
            raise PackError(f"{label}: invalid maxTokens {max_tokens!r}")

    cost = entry.get("cost")
    if not isinstance(cost, dict):
        raise PackError(f"{label}: invalid cost metadata")
    for field in ("input", "output", "cacheRead", "cacheWrite"):
        if not is_number(cost.get(field)):
            raise PackError(f"{label}: invalid cost.{field} {cost.get(field)!r}")

    return model_type, model_id, api


def validate_catalog(raw: bytes) -> tuple[dict, dict, dict]:
    """Validate artifact bytes; return (catalog, structure, counts)."""
    try:
        catalog = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PackError(f"catalog is not valid UTF-8 JSON: {exc}") from exc
    if not isinstance(catalog, dict):
        raise PackError("catalog must be a JSON object keyed by provider")

    expected = expected_providers()
    missing = [p for p in expected if p not in catalog]
    if missing:
        raise PackError(f"catalog is missing providers: {', '.join(missing)}")
    extra = [p for p in catalog if p not in set(expected)]
    if extra:
        raise PackError(f"catalog has unknown providers: {', '.join(sorted(extra))}")

    structure: dict[str, dict[str, str]] = {}
    counts = {"chat": 0, "image": 0, "classifier": 0}
    for provider in expected:
        entries = catalog[provider]
        if not (isinstance(entries, list) and entries):
            raise PackError(f"catalog has no typed model list for provider {provider}")
        provider_structure: dict[str, str] = {}
        for entry in entries:
            model_type, model_id, api = validate_entry(provider, entry)
            identity = f"{model_type}:{model_id}"
            if identity in provider_structure:
                raise PackError(f"{provider}/{identity}: duplicate catalog identity")
            provider_structure[identity] = api
            counts[model_type] += 1
        structure[provider] = dict(sorted(provider_structure.items()))
    return catalog, dict(sorted(structure.items())), counts


def structure_sha256(structure: dict) -> str:
    """Byte-equivalent of Pi's modelDataStructureHash(JSON.stringify(sorted))."""
    normalized = json.dumps(structure, separators=(",", ":"), ensure_ascii=False, sort_keys=True)
    return sha256_bytes(normalized.encode("utf-8"))


def build_manifest(
    raw: bytes, structure: dict, counts: dict, providers: list[str]
) -> dict:
    revision = pinned_revision()
    digest = sha256_bytes(raw)
    if digest != revision.removeprefix("sha256-"):
        raise PackError(
            f"artifact digest {digest} does not match pinned revision {revision}"
        )
    return {
        "bundle_schema_version": BUNDLE_SCHEMA_VERSION,
        "artifact": {
            "kind": "pi-published-model-catalog",
            "url": catalog_url(revision),
            "revision": revision,
            "sha256": digest,
            "file": CATALOG_NAME,
        },
        "source": {
            "pi_package": "@earendil-works/pi-ai@1.3.0",
            "local_origin": ".repo/pi-1.3.0",
            "revision_pin": ".repo/pi-1.3.0/nix/model-catalog.json",
            "tree_evidence": "docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-source-evidence.json",
        },
        "data_schema_version": DATA_SCHEMA_VERSION,
        "min_reader": {"bundle_schema_version": BUNDLE_SCHEMA_VERSION},
        "structure_sha256": structure_sha256(structure),
        "providers": providers,
        "counts": {"providers": len(providers), **counts},
        "generator": {
            "pi_model_data_ts_sha256": sha256_file(PI_MODEL_DATA),
            "pi_hydrate_model_catalog_ts_sha256": sha256_file(PI_HYDRATE),
            "packer": "scripts/pi-catalog/pack.py",
            "packer_sha256": sha256_file(Path(__file__).resolve()),
        },
    }


def emit(raw: bytes, manifest: dict) -> None:
    BUNDLE_DIR.mkdir(parents=True, exist_ok=True)
    manifest_bytes = (json.dumps(manifest, indent=2, ensure_ascii=False, sort_keys=True) + "\n").encode(
        "utf-8"
    )
    with tempfile.TemporaryDirectory(dir=BUNDLE_DIR, prefix=".staging-") as staging:
        staged_catalog = Path(staging) / CATALOG_NAME
        staged_manifest = Path(staging) / MANIFEST_NAME
        staged_catalog.write_bytes(raw)
        staged_manifest.write_bytes(manifest_bytes)
        os.replace(staged_catalog, BUNDLE_DIR / CATALOG_NAME)
        os.replace(staged_manifest, BUNDLE_DIR / MANIFEST_NAME)
    for leftover in BUNDLE_DIR.iterdir():
        if leftover.name not in (CATALOG_NAME, MANIFEST_NAME):
            raise PackError(f"unexpected file in bundle dir: {leftover}")


def check_emitted() -> dict:
    """Validate the already-emitted bundle without rewriting it."""
    catalog_path = BUNDLE_DIR / CATALOG_NAME
    manifest_path = BUNDLE_DIR / MANIFEST_NAME
    if not catalog_path.is_file() or not manifest_path.is_file():
        raise PackError(f"bundle is incomplete under {BUNDLE_DIR}")
    raw = catalog_path.read_bytes()
    _, structure, counts = validate_catalog(raw)
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    expected = build_manifest(raw, structure, counts, expected_providers())
    if manifest != expected:
        raise PackError("emitted manifest does not match the packaged artifact")
    return manifest


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifact", type=Path, help="downloaded pi.dev catalog JSON to package")
    parser.add_argument(
        "--check", action="store_true", help="validate the emitted bundle without rewriting"
    )
    args = parser.parse_args(argv)

    if args.check:
        manifest = check_emitted()
        counts = manifest["counts"]
        print(f"bundle OK: {counts['providers']} providers, structure {manifest['structure_sha256'][:12]}")
        return 0

    if args.artifact is None:
        parser.error("--artifact is required unless --check is given")
    raw = args.artifact.read_bytes()
    _, structure, counts = validate_catalog(raw)
    providers = expected_providers()
    manifest = build_manifest(raw, structure, counts, providers)
    emit(raw, manifest)
    print(
        "packed "
        f"{manifest['artifact']['revision']} -> {BUNDLE_DIR.relative_to(REPO_ROOT)} "
        f"({counts['chat']} chat, {counts['image']} image, {counts['classifier']} classifier)"
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except PackError as exc:
        print(f"pack error: {exc}", file=sys.stderr)
        sys.exit(1)
