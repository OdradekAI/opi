#!/usr/bin/env python3
"""Compare an opi model catalog with a verified Pi reference. Report JSON on stdout."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import re
import sys
from itertools import zip_longest
from pathlib import Path


OPERATIONS = ("chat", "image", "classifier")
MISSING = object()


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError(f"duplicate JSON key {key!r}")
        value[key] = item
    return value


def invalid_constant(value: str):
    raise ValueError(f"invalid JSON constant {value}")


def finite_float(token: str) -> float:
    value = float(token)
    if not math.isfinite(value):
        raise ValueError(f"number out of range: {token}")
    return value


def parse(raw: bytes, path: Path) -> dict:
    try:
        value = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=unique_object,
            parse_constant=invalid_constant,
            parse_float=finite_float,
        )
    except ValueError as exc:
        raise ValueError(f"{path}: {exc}") from exc
    if not isinstance(value, dict) or not value:
        raise ValueError(f"{path}: expected a nonempty JSON object")
    return value


def index(catalog: dict, path: Path) -> dict:
    entries = {}
    for provider, models in catalog.items():
        if not provider or not isinstance(models, list) or not models:
            raise ValueError(f"{path}: expected a nonempty model list for {provider!r}")
        for model in models:
            if not isinstance(model, dict):
                raise ValueError(f"{path}: {provider}: expected a model object")
            operation, model_id, api = (model.get(key) for key in ("type", "id", "api"))
            if operation not in OPERATIONS or not all(
                isinstance(value, str) and value for value in (model_id, api)
            ):
                raise ValueError(f"{path}: {provider}: invalid model type, id or API")
            identity = (provider, operation, model_id)
            if model.get("provider") != provider:
                raise ValueError(f"{path}: {identity}: model provider is {model.get('provider')!r}")
            if identity in entries:
                raise ValueError(f"{path}: duplicate model identity {identity}")
            entries[identity] = model
    return entries


def published_reference(artifact: Path, pin: Path) -> tuple[dict, dict]:
    revision = parse(pin.read_bytes(), pin).get("revision")
    if not isinstance(revision, str) or not re.fullmatch("sha256-[0-9a-f]{64}", revision):
        raise ValueError(f"{pin}: expected a sha256-<digest> revision")
    raw = artifact.read_bytes()
    digest = sha256(raw)
    if f"sha256-{digest}" != revision:
        raise ValueError(f"{artifact}: digest {digest} does not match {revision}")
    return index(parse(raw, artifact), artifact), {
        "kind": "pi-release-artifact",
        "revision": revision,
        "sha256": digest,
    }


def pi_property_order(key: str) -> tuple[int, int | bytes]:
    # Pi sorts strings by UTF-16, then JSON.stringify emits array indices first.
    if re.fullmatch(r"0|[1-9][0-9]{0,9}", key) and int(key) < 2**32 - 1:
        return 0, int(key)
    return 1, key.encode("utf-16-be")


def pi_structure_sha256(entries: dict) -> str:
    structure = {}
    for (provider, operation, model_id), entry in entries.items():
        structure.setdefault(provider, {})[f"{operation}:{model_id}"] = entry["api"]
    normalized = {
        provider: {key: structure[provider][key] for key in sorted(structure[provider], key=pi_property_order)}
        for provider in sorted(structure, key=pi_property_order)
    }
    raw = json.dumps(normalized, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return sha256(raw)


def snapshot_reference(directory: Path) -> tuple[dict, dict]:
    manifest = directory / ".manifest.json"
    manifest_bytes = manifest.read_bytes()
    metadata = parse(manifest_bytes, manifest)
    if type(metadata.get("schemaVersion")) is not int or metadata["schemaVersion"] != 6:
        raise ValueError(f"{manifest}: expected Pi provider data schema 6")
    files = metadata.get("files")
    if not isinstance(files, dict) or not files:
        raise ValueError(f"{manifest}: expected provider file digests")
    shards = {path.name: path for path in directory.glob("*.json") if path != manifest}
    if files.keys() != shards.keys():
        missing = sorted(files.keys() - shards.keys())
        extra = sorted(shards.keys() - files.keys())
        raise ValueError(f"{manifest}: provider files differ (missing: {missing}, extra: {extra})")

    entries = {}
    for name, path in sorted(shards.items()):
        raw = path.read_bytes()
        if sha256(raw) != files[name]:
            raise ValueError(f"{path}: digest does not match {manifest.name}")
        models = []
        for api, group in parse(raw, path).items():
            if not isinstance(group, dict) or not group:
                raise ValueError(f"{path}: expected a nonempty API group for {api!r}")
            for key, model in group.items():
                if (not isinstance(model, dict) or model.get("api") != api
                        or key != f"{model.get('type')}:{model.get('id')}"):
                    raise ValueError(f"{path}: {api}/{key}: model does not match its API or identity key")
                models.append(model)
        entries.update(index({path.stem: models}, path))
    structure_sha256 = pi_structure_sha256(entries)
    if metadata.get("structureHash") != structure_sha256:
        raise ValueError(f"{manifest}: structureHash does not match the provider records")
    return entries, {
        "kind": "pi-provider-snapshot",
        "schema_version": metadata["schemaVersion"],
        "manifest_sha256": sha256(manifest_bytes),
        "structure_sha256": structure_sha256,
    }


def field_changes(opi, reference, path=()):
    if isinstance(opi, dict) and isinstance(reference, dict):
        for key in sorted(opi.keys() | reference.keys()):
            yield from field_changes(opi.get(key, MISSING), reference.get(key, MISSING), (*path, key))
    elif isinstance(opi, list) and isinstance(reference, list):
        for i, (before, after) in enumerate(zip_longest(opi, reference, fillvalue=MISSING)):
            yield from field_changes(before, after, (*path, i))
    elif opi != reference or isinstance(opi, bool) != isinstance(reference, bool):
        change = {"path": list(path)}
        if opi is not MISSING:
            change["opi"] = opi
        if reference is not MISSING:
            change["reference"] = reference
        yield change


def identity_json(identity: tuple[str, str, str]) -> dict:
    provider, operation, model_id = identity
    return {"provider": provider, "type": operation, "id": model_id}


def compare(opi: dict, reference: dict) -> dict:
    only_opi = [identity_json(key) for key in sorted(opi.keys() - reference.keys())]
    only_reference = [identity_json(key) for key in sorted(reference.keys() - opi.keys())]
    shared = opi.keys() & reference.keys()
    changed = []
    for key in sorted(shared):
        fields = list(field_changes(opi[key], reference[key]))
        if fields:
            changed.append({**identity_json(key), "fields": fields})
    return {
        "counts": {
            "opi": len(opi),
            "reference": len(reference),
            "only_opi": len(only_opi),
            "only_reference": len(only_reference),
            "changed": len(changed),
            "equal": len(shared) - len(changed),
        },
        "only_opi": only_opi,
        "only_reference": only_reference,
        "changed": changed,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--opi-catalog", required=True, type=Path, help="opi catalog.json to compare")
    parser.add_argument("--pi-version", required=True, help="version label for the reference")
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--pi-artifact", type=Path, help="published Pi catalog JSON")
    source.add_argument("--pi-provider-data", type=Path, help="Pi provider-data directory with .manifest.json")
    parser.add_argument("--pi-pin", type=Path, help="revision pin for --pi-artifact")
    args = parser.parse_args(argv)
    if (args.pi_artifact is None) != (args.pi_pin is None):
        parser.error("--pi-pin is required with --pi-artifact and only applies to that input")
    if not args.pi_version.strip():
        parser.error("--pi-version must be nonempty")
    try:
        raw = args.opi_catalog.read_bytes()
        opi = index(parse(raw, args.opi_catalog), args.opi_catalog)
        if args.pi_artifact is not None:
            reference, provenance = published_reference(args.pi_artifact, args.pi_pin)
        else:
            reference, provenance = snapshot_reference(args.pi_provider_data)
        report = {
            "opi": {"sha256": sha256(raw)},
            "reference": {**provenance, "pi_version": args.pi_version},
            **compare(opi, reference),
        }
        output = json.dumps(report, ensure_ascii=False, allow_nan=False, indent=2) + "\n"
        sys.stdout.buffer.write(output.encode("utf-8"))
    except (OSError, ValueError) as exc:
        print(f"catalog comparison: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
