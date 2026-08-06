#!/usr/bin/env python3
"""Shared opi-sandbox manifest renderer and archive verifier."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tarfile
import tempfile
import tomllib
import zipfile


EXPECTED_MEMBERS = (
    "package.toml",
    "bin/opi-sandbox",
    "schemas/command-execution-jsonl-v1.schema.json",
    "licenses/LICENSE",
)
MEMBER_LIMITS = {
    "package.toml": 1024 * 1024,
    "bin/opi-sandbox": 64 * 1024 * 1024,
    "schemas/command-execution-jsonl-v1.schema.json": 4 * 1024 * 1024,
    "licenses/LICENSE": 1024 * 1024,
}
LOCK_FIELDS = {
    "manifest_hash",
    "executable_rel_path",
    "executable_sha256",
    "package_version",
    "target",
    "opi_range",
    "protocol",
    "adapter_id",
}
TARGET_RE = re.compile(r"[A-Za-z0-9_][A-Za-z0-9_.-]*\Z")
SHA256_RE = re.compile(r"[0-9a-f]{64}\Z")
SEMVER_RE = re.compile(
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?\Z"
)


class PackageError(Exception):
    pass


def sha256_raw(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_lf(data: bytes) -> str:
    return sha256_raw(data.replace(b"\r", b""))


def parse_semver(version: str) -> tuple[int, int]:
    match = SEMVER_RE.fullmatch(version)
    if match is None:
        raise PackageError(f"invalid workspace package version: {version}")
    prerelease = match.group(4)
    if prerelease is not None:
        for identifier in prerelease.split("."):
            if identifier.isascii() and identifier.isdigit() and len(identifier) > 1 and identifier[0] == "0":
                raise PackageError(f"invalid workspace package version: {version}")
    return int(match.group(1)), int(match.group(2))


def read_workspace_version(path: Path) -> str:
    try:
        parsed = tomllib.loads(path.read_text(encoding="utf-8"))
        version = parsed["workspace"]["package"]["version"]
    except (OSError, UnicodeError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        raise PackageError(f"cannot read workspace package version: {error}") from error
    if not isinstance(version, str):
        raise PackageError("workspace package version must be a string")
    return version


def render(args: argparse.Namespace) -> None:
    manifest_path = Path(args.workspace_manifest)
    template_path = Path(args.template)
    output_path = Path(args.output)
    version = read_workspace_version(manifest_path)
    major, minor = parse_semver(version)
    if TARGET_RE.fullmatch(args.target) is None:
        raise PackageError(f"invalid target triple: {args.target}")
    if SHA256_RE.fullmatch(args.sha256) is None:
        raise PackageError("invalid executable SHA-256")
    opi_range = f">={major}.{minor}.0-0,<{major}.{minor + 1}.0-0"
    try:
        template = template_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PackageError(f"cannot read manifest template: {error}") from error
    replacements = {
        "__PACKAGE_VERSION__": version,
        "__OPI_RANGE__": opi_range,
        "__TARGET__": args.target,
        "__SHA256__": args.sha256,
    }
    for token in replacements:
        if template.count(token) != 1:
            raise PackageError(f"manifest template must contain exactly one {token}")
    rendered = template
    for token, value in replacements.items():
        rendered = rendered.replace(token, value)
    rendered = rendered.replace("\r\n", "\n").replace("\r", "\n")
    try:
        parsed = tomllib.loads(rendered)
    except tomllib.TOMLDecodeError as error:
        raise PackageError(f"rendered manifest is invalid TOML: {error}") from error
    if parsed.get("version") != version or parsed.get("opi_version") != opi_range:
        raise PackageError("rendered manifest changed literal version metadata")
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(rendered.encode("utf-8"))
    if args.metadata_output:
        Path(args.metadata_output).write_text(
            f"{version}\n{opi_range}\n", encoding="utf-8", newline="\n"
        )


def checked_member_name(name: str) -> str:
    if "\\" in name or name.startswith("/"):
        raise PackageError(f"unsafe archive member: {name}")
    path = PurePosixPath(name)
    if path.is_absolute() or any(part in ("", ".", "..") for part in path.parts):
        raise PackageError(f"unsafe archive member: {name}")
    canonical = path.as_posix()
    if name != canonical:
        raise PackageError(f"archive member name is not canonical: {name}")
    return canonical


def canonical_schema_bytes(snapshot_path: Path) -> bytes:
    try:
        snapshot = snapshot_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PackageError(f"verify: cannot read reviewed schema snapshot: {error}") from error
    normalized = snapshot.replace("\r\n", "\n").replace("\r", "\n")
    lines = normalized.splitlines()
    markers = [index for index, line in enumerate(lines) if line == "---"]
    if len(markers) < 2 or markers[1] + 1 >= len(lines):
        raise PackageError("verify: reviewed schema snapshot has an invalid header")
    payload = ("\n".join(lines[markers[1] + 1 :]) + "\n").encode("utf-8")
    try:
        json.loads(payload)
    except json.JSONDecodeError as error:
        raise PackageError(f"verify: reviewed schema snapshot is invalid JSON: {error}") from error
    return payload


def read_archive_members(archive: Path) -> dict[str, bytes]:
    members: dict[str, bytes] = {}
    names: list[str] = []
    if archive.name.endswith(".tar.gz"):
        try:
            with tarfile.open(archive, "r:gz") as source:
                for info in source.getmembers():
                    name = checked_member_name(info.name)
                    names.append(name)
                    if not info.isfile():
                        raise PackageError(f"archive member is not a regular file: {name}")
                    if name == "bin/opi-sandbox" and info.mode & 0o7777 != 0o755:
                        raise PackageError(
                            "archive executable mode must be exactly 0755"
                        )
                    if name not in MEMBER_LIMITS or info.size > MEMBER_LIMITS[name]:
                        raise PackageError(f"unexpected or oversized archive member: {name}")
                    handle = source.extractfile(info)
                    if handle is None:
                        raise PackageError(f"cannot read archive member: {name}")
                    members[name] = handle.read(MEMBER_LIMITS[name] + 1)
        except (OSError, tarfile.TarError) as error:
            raise PackageError(f"cannot read archive: {error}") from error
    elif archive.name.endswith(".zip"):
        try:
            with zipfile.ZipFile(archive, "r") as source:
                for info in source.infolist():
                    name = checked_member_name(info.filename)
                    names.append(name)
                    unix_mode = info.external_attr >> 16
                    if info.is_dir() or (unix_mode and not stat.S_ISREG(unix_mode)):
                        raise PackageError(f"archive member is not a regular file: {name}")
                    if name not in MEMBER_LIMITS or info.file_size > MEMBER_LIMITS[name]:
                        raise PackageError(f"unexpected or oversized archive member: {name}")
                    members[name] = source.read(info)
        except (OSError, zipfile.BadZipFile, RuntimeError) as error:
            raise PackageError(f"cannot read archive: {error}") from error
    else:
        raise PackageError(f"unsupported archive name: {archive.name}")
    if len(names) != len(set(names)):
        raise PackageError("archive contains duplicate members")
    if set(names) != set(EXPECTED_MEMBERS) or len(names) != len(EXPECTED_MEMBERS):
        raise PackageError(f"archive member set mismatch: {sorted(names)}")
    for name, data in members.items():
        if len(data) > MEMBER_LIMITS[name]:
            raise PackageError(f"oversized archive member: {name}")
    return members


def validate_manifest(manifest_bytes: bytes, lock: dict[str, object], target: str) -> None:
    try:
        manifest = tomllib.loads(manifest_bytes.decode("utf-8"))
    except (UnicodeError, tomllib.TOMLDecodeError) as error:
        raise PackageError(f"invalid package.toml: {error}") from error
    if set(manifest) != {"name", "description", "version", "opi_version", "contributions"}:
        raise PackageError("package.toml has unexpected top-level fields")
    if manifest["name"] != "opi-sandbox":
        raise PackageError("package.toml name mismatch")
    if manifest["version"] != lock["package_version"] or manifest["opi_version"] != lock["opi_range"]:
        raise PackageError("package.toml version metadata mismatch")
    contributions = manifest.get("contributions")
    if not isinstance(contributions, dict) or set(contributions) != {"adapters"}:
        raise PackageError("package.toml contributions mismatch")
    adapters = contributions.get("adapters")
    if not isinstance(adapters, list) or len(adapters) != 1 or not isinstance(adapters[0], dict):
        raise PackageError("package.toml must declare exactly one adapter")
    adapter = adapters[0]
    expected = {
        "capability": "command.execute",
        "id": "opi-sandbox",
        "transport": "process-jsonl",
        "command": "bin/opi-sandbox",
        "args": ["backend", "--stdio"],
        "protocol": "command-execution-jsonl-v1",
        "target": target,
        "sha256": lock["executable_sha256"],
        "handshake_timeout_ms": 5000,
        "adapter_config": {},
    }
    if adapter != expected:
        raise PackageError("package.toml adapter identity mismatch")


def verify(args: argparse.Namespace) -> None:
    artifact_dir = Path(args.artifact_dir)
    try:
        raw_target = (artifact_dir / "target").read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise PackageError(f"verify: cannot read target: {error}") from error
    if not raw_target.endswith("\n") or raw_target.count("\n") != 1:
        raise PackageError("verify: target must contain exactly one line")
    target = raw_target[:-1]
    if TARGET_RE.fullmatch(target) is None:
        raise PackageError(f"verify: invalid target: {target}")
    archive = artifact_dir / f"opi-sandbox-{target}{args.archive_suffix}"
    if not archive.is_file():
        raise PackageError(f"verify: expected archive not found: {archive}")
    try:
        lock = tomllib.loads((artifact_dir / "package-lock.toml").read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise PackageError(f"verify: cannot read package-lock.toml: {error}") from error
    if set(lock) != LOCK_FIELDS or not all(isinstance(value, str) for value in lock.values()):
        raise PackageError("verify: package lock has unexpected or non-string fields")
    if lock["target"] != target:
        raise PackageError("verify: lock target mismatch")
    if lock["adapter_id"] != "opi-sandbox" or lock["protocol"] != "command-execution-jsonl-v1":
        raise PackageError("verify: lock adapter identity mismatch")
    if lock["executable_rel_path"] != "bin/opi-sandbox":
        raise PackageError("verify: lock executable path mismatch")
    major, minor = parse_semver(lock["package_version"])
    expected_range = f">={major}.{minor}.0-0,<{major}.{minor + 1}.0-0"
    if lock["opi_range"] != expected_range:
        raise PackageError("verify: lock Opi range mismatch")
    if SHA256_RE.fullmatch(lock["manifest_hash"]) is None or SHA256_RE.fullmatch(lock["executable_sha256"]) is None:
        raise PackageError("verify: invalid locked hash")

    archive_members = read_archive_members(archive)
    with tempfile.TemporaryDirectory(prefix="opi-sandbox-verify-") as temporary:
        extraction = Path(temporary)
        if any(extraction.iterdir()):
            raise PackageError("verify: temporary extraction directory was not empty")
        for name in EXPECTED_MEMBERS:
            destination = extraction / PurePosixPath(name)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(archive_members[name])
        manifest_bytes = (extraction / "package.toml").read_bytes()
        executable_bytes = (extraction / "bin/opi-sandbox").read_bytes()
        if sha256_lf(manifest_bytes) != lock["manifest_hash"]:
            raise PackageError("verify: archive manifest_hash mismatch")
        if sha256_raw(executable_bytes) != lock["executable_sha256"]:
            raise PackageError("verify: archive executable sha mismatch")
        validate_manifest(manifest_bytes, lock, target)
        try:
            schema = json.loads(
                (extraction / "schemas/command-execution-jsonl-v1.schema.json").read_text(
                    encoding="utf-8"
                )
            )
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            raise PackageError(f"verify: invalid protocol schema: {error}") from error
        if schema.get("$id") != "https://odradek.ai/schemas/command-execution-jsonl-v1.json":
            raise PackageError("verify: protocol schema identity mismatch")
        schema_bytes = (
            extraction / "schemas/command-execution-jsonl-v1.schema.json"
        ).read_bytes()
        if schema_bytes != canonical_schema_bytes(Path(args.schema_snapshot)):
            raise PackageError("verify: archive schema does not match the reviewed snapshot")
        try:
            expected_license = Path(args.workspace_license).read_bytes()
        except OSError as error:
            raise PackageError(f"verify: cannot read workspace LICENSE: {error}") from error
        if (extraction / "licenses/LICENSE").read_bytes() != expected_license:
            raise PackageError("verify: archive license mismatch")
    print(
        "verified opi-sandbox archive: "
        f"manifest_hash={lock['manifest_hash']}, executable_sha256={lock['executable_sha256']}"
    )


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    commands = root.add_subparsers(dest="command", required=True)
    render_parser = commands.add_parser("render")
    render_parser.add_argument("--workspace-manifest", required=True)
    render_parser.add_argument("--template", required=True)
    render_parser.add_argument("--target", required=True)
    render_parser.add_argument("--sha256", required=True)
    render_parser.add_argument("--output", required=True)
    render_parser.add_argument("--metadata-output")
    verify_parser = commands.add_parser("verify")
    verify_parser.add_argument("--artifact-dir", required=True)
    verify_parser.add_argument("--archive-suffix", choices=(".tar.gz", ".zip"), required=True)
    verify_parser.add_argument("--workspace-license", required=True)
    verify_parser.add_argument("--schema-snapshot", required=True)
    return root


def main() -> None:
    args = parser().parse_args()
    try:
        if args.command == "render":
            render(args)
        else:
            verify(args)
    except PackageError as error:
        print(f"package-opi-sandbox: {error}", file=sys.stderr)
        raise SystemExit(2 if args.command == "render" else 1) from error


if __name__ == "__main__":
    main()
