#!/usr/bin/env python3
"""Prepare an offline provider catalog candidate from two pinned Pi releases and current opi sources."""

from __future__ import annotations

import argparse
import json
import re
import sys
import tempfile
from pathlib import Path

import pack


def same(left, right):
    # Python equates False with 0, even inside lists and objects.
    return left is right or next(pack.field_changes(left, right), None) is None


def merge_value(base, local, upstream, path, conflicts):
    if same(local, upstream) or same(base, upstream):
        return local
    if same(base, local):
        return upstream
    if (isinstance(local, dict) and isinstance(upstream, dict)
            and (isinstance(base, dict) or (base is pack.MISSING and path))):
        previous = {} if base is pack.MISSING else base
    else:
        # A new model identity belongs to one owner, even when its objects
        # could be merged mechanically. Arrays are ordered values as a whole.
        conflicts.append({
            "path": list(path),
            **{name: value for name, value in (("previous", base), ("local", local), ("upstream", upstream))
               if value is not pack.MISSING},
        })
        return local

    merged = {}
    for key in dict.fromkeys([*local, *upstream]):
        value = merge_value(previous.get(key, pack.MISSING), local.get(key, pack.MISSING),
                            upstream.get(key, pack.MISSING), (*path, key), conflicts)
        if value is not pack.MISSING:
            merged[key] = value
    return merged


def provider_entries(entries, provider):
    return {key: model for key, model in entries.items() if key[0] == provider}


def local_entries(source, raw):
    try:
        models = json.loads(raw.decode("utf-8"), object_pairs_hook=pack.unique_object,
                            parse_constant=pack.invalid_constant, parse_float=pack.finite_float)
    except ValueError as exc:
        raise ValueError(f"{source}: {exc}") from exc
    return pack.index({source.stem: models}, source)


def change_summary(before, after):
    comparison = pack.compare(before, after)
    updated = []
    for row in comparison["changed"]:
        fields = []
        for change in row["fields"]:
            field = {"path": change["path"]}
            if "opi" in change:
                field["before"] = change["opi"]
            if "reference" in change:
                field["after"] = change["reference"]
            fields.append(field)
        updated.append({**row, "fields": fields})
    return {"added": comparison["only_reference"], "removed": comparison["only_opi"],
            "updated": updated}


def merge_provider(previous, local, upstream):
    merged = {}
    conflicts = []
    identical_additions = []
    # Follow release ordering, then preserve the order of opi-only records.
    for identity in dict.fromkeys([*upstream, *local]):
        before = previous.get(identity, pack.MISSING)
        current = local.get(identity, pack.MISSING)
        after = upstream.get(identity, pack.MISSING)
        if before is pack.MISSING and current is not pack.MISSING and same(current, after):
            identical_additions.append(pack.identity_json(identity))
        fields = []
        value = merge_value(before, current, after, (), fields)
        if fields:
            conflicts.append({**pack.identity_json(identity), "fields": fields})
        if value is not pack.MISSING:
            merged[identity] = value
    return merged, conflicts, identical_additions


def publish_candidate(sources, selected, contents, output):
    # Claim a fresh output directory before the single rename publishing the
    # complete 'providers' tree. Existing outputs and inputs stay untouched.
    with tempfile.TemporaryDirectory(dir=output.parent) as staging:
        providers = Path(staging) / "providers"
        providers.mkdir()
        for name, raw in sources.items():
            if name != selected:
                (providers / name).write_bytes(raw)
        if contents is not None:
            (providers / selected).write_bytes(contents)
        output.mkdir()
        try:
            providers.rename(output / "providers")
        except OSError:
            output.rmdir()
            raise


def prepare(args):
    sources = args.opi_sources.resolve(strict=True)
    output = args.output_dir.absolute()
    if output.resolve().is_relative_to(sources):
        raise ValueError("candidate output must be outside the authoring directory")
    if output.exists() or output.is_symlink():
        raise ValueError(f"candidate already exists: {output}")
    provider = args.provider
    if not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", provider):
        raise ValueError(f"invalid provider filename: {provider!r}")
    files = {path.name: path.read_bytes() for path in sorted(sources.glob("*.json"))}
    if not files:
        raise ValueError(f"no provider JSON files in {sources}")
    previous, previous_revision = pack.published_reference(args.old_artifact, args.old_pin)
    upstream, upstream_revision = pack.published_reference(args.new_artifact, args.new_pin)
    previous = provider_entries(previous, provider)
    upstream = provider_entries(upstream, provider)
    name = f"{provider}.json"
    local = local_entries(sources / name, files[name]) if name in files else {}
    if not (local or previous or upstream):
        raise ValueError(f"provider absent from both references and opi sources: {provider}")

    merged, conflicts, identical_additions = merge_provider(previous, local, upstream)
    report = {
        "provider": provider,
        "previous_revision": previous_revision["revision"],
        "upstream_revision": upstream_revision["revision"],
        "source_sha256": pack.sha256(files[name]) if name in files else None,
        "upstream_changes": change_summary(previous, upstream),
        "identical_additions": identical_additions,
        "conflicts": conflicts,
    }
    if conflicts:
        return report, 2

    changes = change_summary(local, merged)
    contents = None
    if merged:
        contents = files[name] if not any(changes.values()) else (
            json.dumps(list(merged.values()), ensure_ascii=False, allow_nan=False, indent=2) + "\n"
        ).encode("utf-8")
    if contents is None and not (files.keys() - {name}):
        raise ValueError("the resulting catalog would contain no providers")
    publish_candidate(files, name, contents, output)
    report.update(candidate_changes=changes, candidate_records=len(merged),
                  candidate_sha256=pack.sha256(contents) if contents is not None else None)
    return report, 0


def main(argv):
    cli = argparse.ArgumentParser(description=__doc__)
    for prefix in ("old", "new"):
        cli.add_argument(f"--{prefix}-artifact", required=True, type=Path, help="published Pi catalog")
        cli.add_argument(f"--{prefix}-pin", required=True, type=Path, help="trusted revision pin for the artifact")
    cli.add_argument("--provider", required=True, help="one provider ID to synchronize")
    cli.add_argument("--opi-sources", required=True, type=Path, help="opi authoring directory")
    cli.add_argument("--output-dir", required=True, type=Path, help="fresh output directory for the providers/ candidate")
    args = cli.parse_args(argv)
    try:
        report, status = prepare(args)
        sys.stdout.buffer.write((json.dumps(report, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
        return status
    except (ValueError, OSError) as exc:
        print(f"catalog sync: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
