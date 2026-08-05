#!/usr/bin/env python3
"""Deterministic artifact audit for opi dogfood runs.

Analyzes saved local artifacts only (no network). Detects:
  - workspace-root path leakage in public NDJSON and session JSONL
    (message/tool records; the by-design session-header `cwd` is skipped);
  - all-zero runtime message timestamps;
  - session_summary.provider_turns != TurnStart count;
  - O(n^2) text_delta lines that carry a redundant cumulative `partial`;
  - report mentions of provider failures with no preserved raw artifact
    and no explicit disclosure phrase.
  - declared commit references that do not resolve to local Git commit objects.

Known limitations (documented, not blocking):
  - Failure-word matching is a deliberately conservative trip-wire; it
    cannot fully distinguish an honest disclosure from a false claim, so
    explicit disclosure phrases are allow-listed.
  - The default session-dir glob matches the opi-run-sandbox shape; pass a
    custom dir layout for other shapes.
"""
import argparse
import hashlib
import json
import os
import pathlib
import re
import stat
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from collections import Counter


FAILURE_WORDS = ["ProviderFailure", "HTTP 429", "HTTP 404", "rate limited"]
DISCLOSURE_PHRASES = [
    "not preserved",
    "overwritten before being preserved",
    "observed-unpreserved",
    "observed live",
    "did not preserve",
]
COMMIT_HEX_RE = re.compile(r"^[0-9a-fA-F]{7,40}$")
TEXT_COMMIT_RE = re.compile(
    r"(?ix)"
    r"\b(?P<label>"
    r"head[ _-]+commit(?:[ _-]+at[ _-]+authoring)?|"
    r"head[ _-]*sha|start[ _-]+commit|verified[ _-]+at[ _-]+commit|"
    r"commit(?:[ _-]+(?:sha|id))?"
    r")\b"
    r"(?:[ \t:=`*()\-]|\r?\n){0,80}"
    r"(?P<reference>[0-9a-f]{7,40})\b"
)
TEXT_COMMIT_RANGE_RE = re.compile(
    r"(?ix)"
    r"\b(?P<label>commit[ _-]+range)\b"
    r"(?:[ \t:=`*()\-]|\r?\n){0,80}"
    r"(?P<start>[0-9a-f]{7,40})\s*\.\.\s*"
    r"(?P<end>[0-9a-f]{7,40})\b"
)


def read_text(path):
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""


def parse_json_lines(path):
    records = []
    for line_no, line in enumerate(read_text(path).splitlines(), start=1):
        if not line.strip():
            continue
        try:
            records.append((line_no, json.loads(line)))
        except json.JSONDecodeError as exc:
            records.append((line_no, {"_parse_error": str(exc), "_raw": line}))
    return records


def _norm(value):
    """Normalize a path-ish string for substring comparison across platforms.

    Collapses any run of forward/backslashes to a single "/" so JSON-escaped
    backslashes ("\\\\") and POSIX slashes compare equal (json.dumps doubles
    every backslash, so a single .replace would leave "//" and miss the match).
    """
    if not value:
        return ""
    norm = os.path.normcase(value)
    norm = re.sub(r"[\\/]+", "/", norm)
    return norm


def _root_forms(workspace_root):
    """All comparable forms of the workspace root to detect in rendered JSON."""
    forms = set()
    if workspace_root:
        forms.add(_norm(workspace_root))
        forms.add(_norm(workspace_root.replace("/", "\\")))
        forms.add(_norm(workspace_root.replace("\\", "/")))
    return [f for f in forms if f]


def _leaks_root(rendered, root_forms_norm):
    norm = _norm(rendered)
    return any(f and f in norm for f in root_forms_norm)


def analyze_ndjson(path, root_forms_norm):
    counts = Counter()
    timestamps = []
    provider_turns = 0
    summary_turns = None
    summary_provider_turns = None
    text_delta_lines = 0
    text_delta_bytes = 0
    partial_mentions = 0
    issues = []

    for line_no, record in parse_json_lines(path):
        if "_parse_error" in record:
            issues.append({
                "code": "invalid_json_line",
                "file": str(path),
                "line": line_no,
                "message": record["_parse_error"],
            })
            continue

        event_type = record.get("type")
        counts[event_type] += 1
        rendered = json.dumps(record, ensure_ascii=False)
        # NDJSON carries no session-cwd field, so any root occurrence here is a leak.
        if _leaks_root(rendered, root_forms_norm):
            issues.append({
                "code": "workspace_root_leak",
                "file": str(path),
                "line": line_no,
                "message": "NDJSON line contains the workspace root",
            })

        if event_type == "Agent":
            inner = record.get("event", {})
            inner_type = inner.get("type")
            counts[f"Agent.{inner_type}"] += 1
            if inner_type == "TurnStart":
                provider_turns += 1
            message = inner.get("message")
            if isinstance(message, dict) and isinstance(message.get("timestamp_ms"), int):
                timestamps.append(message["timestamp_ms"])
            assistant_event = inner.get("assistant_event")
            if isinstance(assistant_event, dict):
                # Wire emits snake_case "text_delta" (#[serde(rename=...)]).
                if assistant_event.get("type") == "text_delta":
                    text_delta_lines += 1
                    text_delta_bytes += len(json.dumps(assistant_event, ensure_ascii=False))
                    if "partial" in assistant_event:
                        partial_mentions += 1
        elif event_type == "session_summary":
            summary_turns = record.get("turns")
            summary_provider_turns = record.get("provider_turns")

    if timestamps and all(ts == 0 for ts in timestamps):
        issues.append({
            "code": "all_zero_timestamps",
            "file": str(path),
            "message": "all observed runtime message timestamps are zero",
        })

    if summary_provider_turns is not None and summary_provider_turns != provider_turns:
        issues.append({
            "code": "provider_turn_mismatch",
            "file": str(path),
            "message": f"summary provider_turns={summary_provider_turns} but TurnStart count={provider_turns}",
        })

    if text_delta_lines >= 50 and partial_mentions == text_delta_lines:
        issues.append({
            "code": "duplicated_text_delta_partials",
            "file": str(path),
            "message": f"{text_delta_lines} text_delta events carry assistant_event.partial (O(n^2) shape)",
        })

    return {
        "file": str(path),
        "counts": dict(counts),
        "provider_turns_seen": provider_turns,
        "summary_turns": summary_turns,
        "summary_provider_turns": summary_provider_turns,
        "timestamp_count": len(timestamps),
        "zero_timestamp_count": sum(1 for ts in timestamps if ts == 0),
        "text_delta_lines": text_delta_lines,
        "text_delta_bytes": text_delta_bytes,
        "partial_mentions": partial_mentions,
        "issues": issues,
    }


def analyze_session(path, root_forms_norm):
    issues = []
    records = parse_json_lines(path)
    for line_no, record in records:
        # The session header (type == "session") legitimately stores cwd.
        if isinstance(record, dict) and record.get("type") == "session":
            continue
        rendered = json.dumps(record, ensure_ascii=False)
        if _leaks_root(rendered, root_forms_norm):
            issues.append({
                "code": "session_workspace_root_leak",
                "file": str(path),
                "line": line_no,
                "message": "session message/leaf record contains the workspace root",
            })
    return {"file": str(path), "records": len(records), "issues": issues}


def analyze_failure_evidence(artifact_dir):
    report_blob = "\n".join(
        read_text(path)
        for path in [artifact_dir / "RUN_SUMMARY.md", artifact_dir / "REVIEW_REPORT.md"]
    )
    report_lower = report_blob.lower()
    mentioned = [w for w in FAILURE_WORDS if w.lower() in report_lower]
    disclosed = any(phrase in report_lower for phrase in DISCLOSURE_PHRASES)

    preserved = []
    for path in artifact_dir.glob("run*.ndjson"):
        body = read_text(path).lower()
        preserved.extend(w for w in FAILURE_WORDS if w.lower() in body)
    stderr = read_text(artifact_dir / "run.stderr.log").lower()
    preserved.extend(w for w in FAILURE_WORDS if w.lower() in stderr)

    issues = []
    if mentioned and not preserved and not disclosed:
        issues.append({
            "code": "failure_claim_without_preserved_artifact",
            "message": (
                "report mentions failure markers "
                f"{sorted(set(mentioned))} but preserved run artifacts do not "
                "contain them and no disclosure phrase was found"
            ),
        })
    return {
        "mentioned": sorted(set(mentioned)),
        "preserved": sorted(set(preserved)),
        "disclosed": disclosed,
        "issues": issues,
    }


def _snake_case(value):
    return re.sub(r"(?<!^)(?=[A-Z])", "_", value).lower().replace("-", "_")


def _is_commit_metadata_key(key):
    normalized = _snake_case(key)
    return (
        normalized in {"commit", "commit_sha", "head_sha", "commits"}
        or normalized.endswith("_commit")
        or normalized.endswith("_commit_sha")
        or normalized.endswith("_commits")
    )


def _json_commit_references(value, path, pointer="$"):
    references = []
    if isinstance(value, dict):
        for key, child in value.items():
            child_pointer = f"{pointer}.{key}"
            if (
                _is_commit_metadata_key(key)
                and isinstance(child, str)
                and COMMIT_HEX_RE.fullmatch(child)
            ):
                references.append({
                    "file": str(path),
                    "declaration": child_pointer,
                    "reference": child,
                })
            elif _is_commit_metadata_key(key) and isinstance(child, list):
                for index, item in enumerate(child):
                    if isinstance(item, str) and COMMIT_HEX_RE.fullmatch(item):
                        references.append({
                            "file": str(path),
                            "declaration": f"{child_pointer}[{index}]",
                            "reference": item,
                        })
            references.extend(_json_commit_references(child, path, child_pointer))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            references.extend(
                _json_commit_references(child, path, f"{pointer}[{index}]")
            )
    return references


def _text_commit_references(text, path):
    references = []
    occupied = []
    for match in TEXT_COMMIT_RANGE_RE.finditer(text):
        occupied.append(match.span())
        for group in ["start", "end"]:
            references.append({
                "file": str(path),
                "line": text.count("\n", 0, match.start(group)) + 1,
                "declaration": match.group("label"),
                "reference": match.group(group),
            })
    for match in TEXT_COMMIT_RE.finditer(text):
        if any(start <= match.start() < end for start, end in occupied):
            continue
        references.append({
            "file": str(path),
            "line": text.count("\n", 0, match.start("reference")) + 1,
            "declaration": match.group("label"),
            "reference": match.group("reference"),
        })
    return references


def collect_commit_references(artifact_dir):
    references = []
    for path in sorted(p for p in artifact_dir.rglob("*") if p.is_file()):
        text = read_text(path)
        if path.suffix.lower() == ".json":
            try:
                value = json.loads(text)
            except json.JSONDecodeError:
                value = None
            if value is not None:
                references.extend(_json_commit_references(value, path))
        references.extend(_text_commit_references(text, path))
    return references


def analyze_commit_references(artifact_dir, workspace_root):
    references = collect_commit_references(artifact_dir)
    issues = []
    for reference in references:
        commit = reference["reference"]
        lookup = subprocess.run(
            ["git", "-C", str(workspace_root), "cat-file", "-t", commit],
            capture_output=True,
            check=False,
            encoding="utf-8",
            errors="replace",
        )
        if lookup.returncode != 0 or lookup.stdout.strip() != "commit":
            issue = {
                "code": "missing_commit_reference",
                **reference,
                "message": (
                    f"declared commit reference {commit} does not resolve to "
                    "a local Git commit object"
                ),
            }
            issues.append(issue)
    return references, issues


def _session_files(artifact_dir):
    # Broad glob: top-level sessions/, sibling attempts, and nested shapes.
    seen = set()
    for path in artifact_dir.rglob("sessions*"):
        if path.is_dir():
            for f in path.glob("*.jsonl"):
                seen.add(f.resolve())
    return sorted(seen)


# ---------------------------------------------------------------------------
# Phase 16 task 16.15.2: release-archive audit mode (--release).
#
# Validates the published native opi-sandbox topology (SC16-12b): native target
# identity, archive layout, extracted-binary provenance, smoke evidence, and
# complete non-skipped / non-zero-test Linux/macOS/Windows evidence. Rejects
# absent, wrong-target, workspace-only, skipped, or zero-test evidence.
#
# Evidence directory layout (one bundle per supported target):
#   <dir>/linux/<target>/ target, package-lock.toml, the target archive, and
#                         direct + backend smoke markers bound to its SHA-256.
#   <dir>/macos/<target>/ same shape for both apple-darwin triples.
#   <dir>/windows/ a *.txt/*.log evidence file reporting doctor supported=false
#                 plus a passing unsupported-posture test result; NO extracted
#                 archive (16.14.2 unsupported posture).
# ---------------------------------------------------------------------------

# Native opi-sandbox target families that ship an archive, keyed by the evidence
# platform dir. Windows is absent: no native opi-sandbox confinement, so no
# archive is produced.
NATIVE_ARCHIVE_PLATFORMS = {
    "linux": "-unknown-linux-gnu",
    "macos": "-apple-darwin",
}
NATIVE_ARCHIVE_TARGETS = {
    "linux": ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"],
    "macos": ["x86_64-apple-darwin", "aarch64-apple-darwin"],
}
ARCHIVE_MEMBER_LIMITS = {
    "package.toml": 1024 * 1024,
    "bin/opi-sandbox": 64 * 1024 * 1024,
    "schemas/command-execution-jsonl-v1.schema.json": 4 * 1024 * 1024,
    "licenses/LICENSE": 1024 * 1024,
}
ARCHIVE_TOTAL_LIMIT = sum(ARCHIVE_MEMBER_LIMITS.values())
KNOWN_MANIFEST_FIELDS = {"name", "description", "version", "opi_version", "contributions"}
KNOWN_CONTRIBUTIONS_FIELDS = {"adapters"}
KNOWN_ADAPTER_FIELDS = {
    "capability", "id", "transport", "command", "args", "protocol", "target",
    "sha256", "handshake_timeout_ms", "adapter_config",
}
SMOKE_OK_RE = re.compile(r"opi-sandbox-smoke:\s*OK")
DIRECT_SMOKE_RE = re.compile(
    r"opi-sandbox-direct-smoke:\s*OK\s+archive_sha256=([0-9a-f]{64})"
)
BACKEND_SMOKE_RE = re.compile(
    r"opi-sandbox-backend-smoke:\s*OK\s+archive_sha256=([0-9a-f]{64})"
)
CARGO_PASS_RE = re.compile(r"test result: ok\. ([1-9][0-9]*) passed; 0 failed; 0 ignored")
CARGO_SKIPPED_RE = re.compile(r"test result: ok\. \d+ passed; 0 failed; ([1-9][0-9]*) ignored")
CARGO_ZERO_RE = re.compile(r"test result: ok\. 0 passed; 0 failed; 0 ignored")
EVIDENCE_FAILURE_RE = re.compile(
    r"test result: FAILED|error: test failed|error: could not compile|"
    r"opi-sandbox-(?:direct|backend)-smoke:\s*(?:FAIL|FAILED)|AssertionError|Traceback"
)
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


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _bundle_evidence_text(bundle):
    """Concatenate text/log evidence, including a packager's smoke/ subtree."""
    parts = []
    for entry in sorted(bundle.rglob("*")):
        if entry.is_file() and entry.suffix.lower() in {".txt", ".log"}:
            parts.append(read_text(entry))
    return "\n".join(parts)


def _classify_evidence(text, platform, issues):
    """Classify evidence, giving any failure/skip/zero marker precedence."""
    if EVIDENCE_FAILURE_RE.search(text):
        issues.append({
            "code": "failed_evidence",
            "platform": platform,
            "message": f"{platform} evidence records a failed run",
        })
        return False
    if CARGO_SKIPPED_RE.search(text):
        issues.append({
            "code": "skipped_evidence",
            "platform": platform,
            "message": f"{platform} evidence has ignored/skipped tests",
        })
        return False
    if CARGO_ZERO_RE.search(text):
        issues.append({
            "code": "zero_test_evidence",
            "platform": platform,
            "message": f"{platform} evidence records a zero-test run",
        })
        return False
    if SMOKE_OK_RE.search(text) or CARGO_PASS_RE.search(text):
        return True
    issues.append({
        "code": "zero_test_evidence",
        "platform": platform,
        "message": f"{platform} evidence has no passing smoke/test marker",
    })
    return False


def _archive_target(path):
    name = path.name
    prefix = "opi-sandbox-"
    if name.startswith(prefix) and name.endswith(".tar.gz"):
        return name[len(prefix):-len(".tar.gz")]
    if name.startswith(prefix) and name.endswith(".zip"):
        return name[len(prefix):-len(".zip")]
    return None


def _safe_member_name(name):
    if "\\" in name:
        return None
    while name.startswith("./"):
        name = name[2:]
    if name in {"", "."}:
        return ""
    path = pathlib.PurePosixPath(name)
    if path.is_absolute() or ".." in path.parts or (path.parts and ":" in path.parts[0]):
        return None
    return path.as_posix().rstrip("/")


def _extract_owned_archive(archive, destination):
    """Extract the exact distribution-wrapper layout into an owned empty dir."""
    expected_files = set(ARCHIVE_MEMBER_LIMITS)
    allowed_dirs = {"", "bin", "schemas", "licenses"}
    seen = set()
    destination.mkdir()

    total_written = 0

    def write_member(name, source, declared_size, mode):
        nonlocal total_written
        if name not in expected_files or name in seen:
            raise ValueError(f"unexpected or duplicate archive member: {name}")
        limit = ARCHIVE_MEMBER_LIMITS[name]
        if declared_size < 0 or declared_size > limit or total_written + declared_size > ARCHIVE_TOTAL_LIMIT:
            raise ValueError(f"archive member exceeds extraction limit: {name}")
        if name == "bin/opi-sandbox" and mode & 0o111 == 0:
            raise ValueError("archive executable has no Unix execute bit")
        seen.add(name)
        output = destination / pathlib.PurePosixPath(name)
        output.parent.mkdir(parents=True, exist_ok=True)
        written = 0
        with open(output, "wb") as handle:
            while True:
                chunk = source.read(65536)
                if not chunk:
                    break
                written += len(chunk)
                if written > limit or total_written + written > ARCHIVE_TOTAL_LIMIT:
                    raise ValueError(f"archive member exceeds extraction limit: {name}")
                handle.write(chunk)
        if written != declared_size:
            raise ValueError(f"archive member size mismatch: {name}")
        total_written += written

    if archive.name.endswith(".tar.gz"):
        # Stream headers and payloads so an archive with an excessive member
        # count is rejected at the first unexpected entry without first
        # materializing its complete table of contents.
        with tarfile.open(archive, "r|gz") as source:
            for member in source:
                name = _safe_member_name(member.name)
                if name is None:
                    raise ValueError(f"unsafe archive member: {member.name}")
                if member.isdir():
                    if name not in allowed_dirs:
                        raise ValueError(f"unexpected archive directory: {member.name}")
                    continue
                if not member.isfile():
                    raise ValueError(f"non-regular archive member: {member.name}")
                extracted = source.extractfile(member)
                if extracted is None:
                    raise ValueError(f"unreadable archive member: {member.name}")
                with extracted:
                    write_member(name, extracted, member.size, member.mode)
    elif archive.name.endswith(".zip"):
        with zipfile.ZipFile(archive) as source:
            for member in source.infolist():
                name = _safe_member_name(member.filename)
                if name is None:
                    raise ValueError(f"unsafe archive member: {member.filename}")
                if member.is_dir():
                    if name not in allowed_dirs:
                        raise ValueError(f"unexpected archive directory: {member.filename}")
                    continue
                mode = (member.external_attr >> 16) & 0o170000
                if mode == stat.S_IFLNK:
                    raise ValueError(f"non-regular archive member: {member.filename}")
                with source.open(member) as extracted:
                    unix_mode = (member.external_attr >> 16) & 0o7777
                    write_member(name, extracted, member.file_size, unix_mode)
    else:
        raise ValueError("unsupported archive format")

    if seen != expected_files:
        raise ValueError(f"archive layout is {sorted(seen)}, expected {sorted(expected_files)}")


def _validate_archive_assets(extracted):
    schema_path = extracted / "schemas" / "command-execution-jsonl-v1.schema.json"
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise ValueError(f"invalid packaged protocol schema: {error}") from error
    if (
        not isinstance(schema, dict)
        or schema.get("$id")
        != "https://odradek.ai/schemas/command-execution-jsonl-v1.json"
        or "command-execution-jsonl-v1" not in schema.get("$comment", "")
        or not isinstance(schema.get("oneOf"), list)
        or len(schema["oneOf"]) != 2
        or not isinstance(schema.get("$defs"), dict)
        or not {"HostToBackend", "BackendToHost"}.issubset(schema["$defs"])
    ):
        raise ValueError("packaged protocol schema has the wrong identity or shape")
    snapshot_path = (
        pathlib.Path(__file__).resolve().parent.parent
        / "crates"
        / "opi-protocol"
        / "tests"
        / "snapshots"
        / "execution_v1_schema__schema_v1.snap"
    )
    snapshot_lines = snapshot_path.read_text(encoding="utf-8").splitlines()
    markers = [index for index, line in enumerate(snapshot_lines) if line == "---"]
    if len(markers) < 2:
        raise ValueError("repository protocol schema snapshot has an invalid header")
    expected_schema = ("\n".join(snapshot_lines[markers[1] + 1:]) + "\n").encode()
    if schema_path.read_bytes() != expected_schema:
        raise ValueError("packaged protocol schema does not match the reviewed snapshot")

    packaged_license = (extracted / "licenses" / "LICENSE").read_bytes()
    repository_license = (
        pathlib.Path(__file__).resolve().parent.parent / "LICENSE"
    ).read_bytes()
    if packaged_license != repository_license:
        raise ValueError("packaged license does not match the repository LICENSE")


def _parse_manifest(path, platform, issues):
    try:
        raw = path.read_bytes()
        text = raw.decode("utf-8")
        if re.search(r"__[A-Z_]+__", text):
            raise ValueError("manifest contains an unresolved placeholder")
        manifest = tomllib.loads(text)
        unknown_manifest = set(manifest) - KNOWN_MANIFEST_FIELDS
        if unknown_manifest:
            raise ValueError(f"manifest has unknown fields: {sorted(unknown_manifest)}")
        contributions = manifest.get("contributions", {})
        if not isinstance(contributions, dict):
            raise ValueError("manifest contributions must be a table")
        unknown_contributions = set(contributions) - KNOWN_CONTRIBUTIONS_FIELDS
        if unknown_contributions:
            raise ValueError(
                f"manifest contributions has unknown fields: {sorted(unknown_contributions)}"
            )
        adapters = contributions.get("adapters", [])
        if len(adapters) != 1:
            raise ValueError("manifest must declare exactly one adapter")
        adapter = adapters[0]
        unknown_adapter = set(adapter) - KNOWN_ADAPTER_FIELDS
        if unknown_adapter:
            raise ValueError(f"adapter has unknown fields: {sorted(unknown_adapter)}")
        required = {
            "name": manifest.get("name"),
            "version": manifest.get("version"),
            "opi_version": manifest.get("opi_version"),
            "capability": adapter.get("capability"),
            "id": adapter.get("id"),
            "transport": adapter.get("transport"),
            "command": adapter.get("command"),
            "args": adapter.get("args"),
            "protocol": adapter.get("protocol"),
            "target": adapter.get("target"),
            "sha256": adapter.get("sha256"),
            "handshake_timeout_ms": adapter.get("handshake_timeout_ms"),
            "adapter_config": adapter.get("adapter_config"),
        }
        expected = {
            "name": "opi-sandbox",
            "capability": "command.execute",
            "id": "opi-sandbox",
            "transport": "process-jsonl",
            "command": "bin/opi-sandbox",
            "args": ["backend", "--stdio"],
            "protocol": "command-execution-jsonl-v1",
            "handshake_timeout_ms": 5000,
            "adapter_config": {},
        }
        for key, value in expected.items():
            if required[key] != value:
                raise ValueError(f"manifest {key} is {required[key]!r}, expected {value!r}")
        version_match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:[-+].*)?", required["version"] or "")
        if not version_match:
            raise ValueError("manifest version is not semver")
        compatible = ">=%s.%s,<%s.%d" % (
            version_match.group(1), version_match.group(2),
            version_match.group(1), int(version_match.group(2)) + 1,
        )
        if required["opi_version"] != compatible:
            raise ValueError("manifest opi_version is not the package minor compatibility range")
        if not re.fullmatch(r"[0-9a-f]{64}", required["sha256"] or ""):
            raise ValueError("manifest sha256 is not lowercase SHA-256")
        if not isinstance(required["target"], str) or not required["target"]:
            raise ValueError("manifest target is empty or non-string")
        return raw, required
    except (OSError, UnicodeError, tomllib.TOMLDecodeError, TypeError, ValueError) as error:
        issues.append({
            "code": "invalid_package_manifest",
            "platform": platform,
            "message": f"{platform} package manifest is invalid: {error}",
        })
        return None, None


def _parse_lock(path, platform, issues):
    try:
        lock = tomllib.loads(path.read_text(encoding="utf-8"))
        if set(lock) != LOCK_FIELDS or not all(isinstance(lock[key], str) for key in LOCK_FIELDS):
            raise ValueError("lock must contain exactly the eight string LockMaterial fields")
        return lock
    except (OSError, UnicodeError, tomllib.TOMLDecodeError, KeyError, ValueError) as error:
        issues.append({
            "code": "invalid_package_lock",
            "platform": platform,
            "message": f"{platform} package lock is invalid: {error}",
        })
        return None


def _audit_native_smoke(bundle, platform, archive_sha, issues):
    text = _bundle_evidence_text(bundle)
    if not _classify_evidence(text, platform, issues):
        return
    direct = DIRECT_SMOKE_RE.findall(text)
    backend = BACKEND_SMOKE_RE.findall(text)
    if not direct or not backend:
        issues.append({
            "code": "missing_smoke_evidence",
            "platform": platform,
            "message": f"{platform} lacks separate direct and backend smoke markers",
        })
        return
    if any(value != archive_sha for value in direct + backend):
        issues.append({
            "code": "archive_digest_mismatch",
            "platform": platform,
            "message": f"{platform} smoke evidence is not bound to archive {archive_sha}",
        })


def _audit_native_bundle(root, platform, target_suffix, issues, expected_target=None):
    bundle = root / platform
    label = platform
    if expected_target is not None:
        bundle = bundle / expected_target
        label = expected_target
    if not bundle.is_dir():
        issues.append({
            "code": "missing_platform_evidence",
            "platform": label,
            "message": f"missing native evidence bundle for {label}",
        })
        return
    if (bundle / "extracted").exists():
        issues.append({
            "code": "caller_prepared_extracted_tree",
            "platform": platform,
            "message": f"{platform} supplies a caller-prepared extracted tree",
        })
    target_file = read_text(bundle / "target").strip()
    if not target_file:
        issues.append({
            "code": "missing_platform_evidence",
            "platform": platform,
            "message": f"{platform} bundle missing target file",
        })
    archives = [
        entry for entry in bundle.iterdir()
        if entry.is_file() and _archive_target(entry) is not None
    ]
    if not archives:
        issues.append({
            "code": "missing_archive",
            "platform": platform,
            "message": f"{platform} bundle has no opi-sandbox archive",
        })
        _classify_evidence(_bundle_evidence_text(bundle), platform, issues)
        return
    if len(archives) != 1:
        issues.append({
            "code": "invalid_archive_layout",
            "platform": platform,
            "message": f"{platform} bundle has {len(archives)} native archives; expected one",
        })
        return
    archive = archives[0]
    archive_target = _archive_target(archive)
    if not archive.name.endswith(".tar.gz"):
        issues.append({
            "code": "invalid_archive_layout",
            "platform": platform,
            "message": f"{platform} native archive must use .tar.gz",
        })
    if (not archive_target or "windows" in archive_target or
            "pc-windows" in archive_target or not archive_target.endswith(target_suffix)
            or (expected_target is not None and archive_target != expected_target)):
        issues.append({
            "code": "wrong_target_identity",
            "platform": platform,
            "message": f"{platform} archive target {archive_target} is not a native {platform} triple",
        })
    if target_file and archive_target != target_file:
        issues.append({
            "code": "wrong_target_identity",
            "platform": platform,
            "message": f"{platform} target file {target_file} != archive target {archive_target}",
        })

    archive_sha = sha256_file(archive)
    with tempfile.TemporaryDirectory(prefix="opi-artifact-audit-") as owned:
        extracted = pathlib.Path(owned) / "extracted"
        try:
            _extract_owned_archive(archive, extracted)
            _validate_archive_assets(extracted)
        except (OSError, tarfile.TarError, zipfile.BadZipFile, ValueError) as error:
            issues.append({
                "code": "invalid_archive_layout",
                "platform": platform,
                "message": f"{platform} archive is invalid: {error}",
            })
            _audit_native_smoke(bundle, platform, archive_sha, issues)
            return

        extracted_bin = extracted / "bin" / "opi-sandbox"
        extracted_manifest = extracted / "package.toml"
        manifest_raw, manifest = _parse_manifest(extracted_manifest, platform, issues)
        lock = _parse_lock(bundle / "package-lock.toml", platform, issues)
        if manifest is not None and lock is not None:
            manifest_hash = hashlib.sha256(manifest_raw.replace(b"\r", b"")).hexdigest()
            actual_sha = sha256_file(extracted_bin)
            expected_lock = {
                "manifest_hash": manifest_hash,
                "executable_rel_path": "bin/opi-sandbox",
                "executable_sha256": actual_sha,
                "package_version": manifest["version"],
                "target": manifest["target"],
                "opi_range": manifest["opi_version"],
                "protocol": manifest["protocol"],
                "adapter_id": manifest["id"],
            }
            for key, expected in expected_lock.items():
                if lock[key] != expected:
                    issues.append({
                        "code": "provenance_mismatch",
                        "platform": platform,
                        "message": f"{platform} lock {key}={lock[key]!r} != {expected!r}",
                    })
            if manifest["sha256"] != actual_sha:
                issues.append({
                    "code": "provenance_mismatch",
                    "platform": platform,
                    "message": f"{platform} manifest executable sha does not match archive bytes",
                })
            if manifest["target"] != archive_target:
                issues.append({
                    "code": "wrong_target_identity",
                    "platform": platform,
                    "message": f"{platform} manifest target {manifest['target']} != {archive_target}",
                })
    _audit_native_smoke(bundle, platform, archive_sha, issues)


def _audit_windows_bundle(root, issues):
    bundle = root / "windows"
    if not bundle.is_dir():
        issues.append({
            "code": "missing_platform_evidence",
            "platform": "windows",
            "message": "missing windows unsupported-posture evidence bundle",
        })
        return
    windows_archives = [
        entry for entry in bundle.iterdir()
        if entry.is_file() and entry.name.startswith("opi-sandbox-")
        and (entry.name.endswith(".tar.gz") or entry.name.endswith(".zip"))
    ]
    if (bundle / "extracted").exists() or windows_archives:
        issues.append({
            "code": "wrong_target_identity",
            "platform": "windows",
            "message": "Windows must not ship an opi-sandbox archive",
        })
    doctor_path = bundle / "unsupported.log"
    try:
        doctor = json.loads(read_text(doctor_path))
        doctor_is_unsupported = (
            isinstance(doctor, dict)
            and doctor.get("schema_version") == 1
            and doctor.get("target") == "windows"
            and doctor.get("supported") is False
        )
    except (OSError, ValueError, TypeError):
        doctor_is_unsupported = False
    if not doctor_is_unsupported:
        issues.append({
            "code": "wrong_target_identity",
            "platform": "windows",
            "message": "Windows doctor JSON does not report supported=false for target=windows",
        })
    text = _bundle_evidence_text(bundle)
    _classify_evidence(text, "windows", issues)


def audit_release_evidence(artifact_dir):
    issues = []
    for platform, target_suffix in NATIVE_ARCHIVE_PLATFORMS.items():
        platform_root = artifact_dir / platform
        targets = NATIVE_ARCHIVE_TARGETS[platform]
        if any((platform_root / target).is_dir() for target in targets):
            for target in targets:
                _audit_native_bundle(
                    artifact_dir, platform, target_suffix, issues, expected_target=target
                )
        else:
            # Keep inspecting legacy flat evidence so defects remain
            # attributable, but it can never satisfy the four-target gate.
            _audit_native_bundle(artifact_dir, platform, target_suffix, issues)
            target_file = read_text(platform_root / "target").strip()
            for target in targets:
                if target != target_file:
                    issues.append({
                        "code": "missing_platform_evidence",
                        "platform": target,
                        "message": f"missing native evidence bundle for {target}",
                    })
    _audit_windows_bundle(artifact_dir, issues)
    platforms = (
        sorted(p.name for p in artifact_dir.iterdir() if p.is_dir())
        if artifact_dir.is_dir()
        else []
    )
    return {
        "artifact_dir": str(artifact_dir),
        "mode": "release",
        "platforms": platforms,
        "issues": issues,
        "ok": not issues,
    }


# ---------------------------------------------------------------------------
# Phase 16 task 16.16.3: phase-exit evidence mode (--phase-exit).
#
# Genuinely validates the preserved Phase 16 phase-exit evidence (SC16-15b and
# the 16.16.3 smoke addendum) against the claimed categories and rejects absent,
# skipped, zero-test, wrong-target, and workspace-only evidence. Evidence
# shapes preservable off-CI:
#   windows/    doctor supported=false plus a genuine pass marker (no archive)
#   linux/      native smoke evidence with a genuine pass marker; when a
#               packaged extracted archive is preserved it is additionally
#               validated for target identity and executable-sha provenance
#   macos/      a preserved CI log carrying a genuine pass marker plus a
#               `source` provenance note naming the CI run/job (the archive
#               itself is CI-produced and pinned by the release-topology gate)
#   six-target/ one preserved `cargo check --target` log per release triple;
#               each log must record its outcome (a green `Finished` check or an
#               explicit `error[` compiler record), never a blank/absent log
#   gates/      preserved workspace gate evidence (doc guards, product captures,
#               crate-boundary, packaging, release-topology) with a pass marker
# ---------------------------------------------------------------------------

SIX_TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
]

GATE_PASS_RE = re.compile(r"test result: ok\. ([1-9][0-9]*) passed; 0 failed")


def _audit_phase_exit_native(root, platform, target_suffix, issues):
    """Phase exit has the same authenticated native-archive requirement."""
    platform_root = root / platform
    targets = NATIVE_ARCHIVE_TARGETS[platform]
    if any((platform_root / target).is_dir() for target in targets):
        for target in targets:
            _audit_native_bundle(root, platform, target_suffix, issues, expected_target=target)
    else:
        _audit_native_bundle(root, platform, target_suffix, issues)
        target_file = read_text(platform_root / "target").strip()
        for target in targets:
            if target != target_file:
                issues.append({
                    "code": "missing_platform_evidence",
                    "platform": target,
                    "message": f"missing native evidence bundle for {target}",
                })


# Gate categories the DoD's final-artifact-audit clause names, keyed by a
# filename marker the preserved capture must carry. A category with no
# preserved pass-marked capture is missing evidence.
GATE_CATEGORIES = {
    "doc-guards": "doc-guard",
    "crate-boundary": "crate-boundary",
    "packaging": "packaging",
    "release-topology": "release-topology",
    "workspace-test": "workspace-test",
    "doctest": "doctest",
    "fmt": "fmt",
    "clippy": "clippy",
    "rustdoc": "rustdoc",
}

GATE_CLEAN_RE = re.compile(r"(PASS|--check: clean|Finished `[^`]+` profile)")

# Outcome markers that prove a preserved gate run FAILED; any capture carrying
# one is rejected even when it also contains passing lines.
GATE_FAILURE_RE = re.compile(
    r"test result: FAILED|error: test failed|error: could not compile|error\["
)

# Test-based gate categories require a NON-ZERO cargo pass; the Finished fallback
# is reserved for the non-test gates (fmt/clippy/rustdoc) whose clean output has
# no test-result line.
GATE_TEST_CATEGORIES = {
    "doc-guards",
    "crate-boundary",
    "packaging",
    "release-topology",
    "workspace-test",
    "doctest",
}


def _gate_pass_marker(category, text):
    if GATE_PASS_RE.search(text):
        return True
    if category in GATE_TEST_CATEGORIES:
        # A test-based gate must prove a non-zero pass; 0-passed/Finished-only
        # evidence is zero-test.
        return False
    return bool(GATE_CLEAN_RE.search(text))


def _audit_six_target_bundle(root, issues):
    six = root / "six-target"
    if not six.is_dir():
        issues.append({
            "code": "missing_six_target_evidence",
            "message": "missing six-target evidence bundle",
        })
        return
    logs = {
        entry.name: read_text(entry)
        for entry in sorted(six.iterdir())
        if entry.is_file() and entry.suffix.lower() in {".txt", ".log"}
    }
    if not logs:
        issues.append({
            "code": "zero_test_evidence",
            "message": "six-target bundle has no preserved logs",
        })
        return
    if not (six / "source").is_file():
        issues.append({
            "code": "missing_provenance",
            "message": "six-target bundle lacks a `source` provenance note naming the "
                "CI run / local runner that produced each triple log",
        })
    for triple in SIX_TARGETS:
        matches = [
            text
            for text in logs.values()
            if re.search(rf"cargo check.*--target {re.escape(triple)}\b", text)
        ]
        if not matches:
            issues.append({
                "code": "missing_target_evidence",
                "message": f"no preserved cargo-check log for {triple}",
            })
            continue
        text = "\n".join(matches)
        if "error[" in text:
            # A compiler failure means that triple's check did NOT pass; the
            # six-target gate is not green and the audit must flag it.
            issues.append({
                "code": "failed_target_evidence",
                "message": f"{triple} cargo check recorded a compiler failure",
            })
            continue
        if "Finished" not in text:
            issues.append({
                "code": "ambiguous_target_evidence",
                "message": f"{triple} log records neither a Finished check nor a compiler error",
            })


def _audit_gates_bundle(root, issues):
    gates = root / "gates"
    if not gates.is_dir():
        issues.append({
            "code": "missing_gate_evidence",
            "message": "missing workspace gate evidence bundle",
        })
        return
    by_name = {
        entry.name: read_text(entry)
        for entry in sorted(gates.iterdir())
        if entry.is_file() and entry.suffix.lower() in {".txt", ".log"}
    }
    if not by_name:
        issues.append({
            "code": "zero_test_evidence",
            "message": "gates bundle has no preserved pass-marked captures",
        })
        return
    for category, marker in GATE_CATEGORIES.items():
        captures = [n for n in by_name if marker in n]
        if not captures:
            issues.append({
                "code": "missing_gate_evidence",
                "message": f"no preserved gate capture for category `{category}` "
                    f"(filename marker `{marker}`)",
            })
            continue
        for name in captures:
            text = by_name[name]
            if GATE_FAILURE_RE.search(text):
                issues.append({
                    "code": "failed_gate_evidence",
                    "message": f"gates/{name} records a failed gate run for `{category}`",
                })
                continue
            if not _gate_pass_marker(category, text):
                issues.append({
                    "code": "zero_test_evidence",
                    "message": f"gates/{name} lacks a genuine pass marker for `{category}`",
                })


def audit_phase_exit_evidence(artifact_dir):
    issues = []
    for platform, target_suffix in NATIVE_ARCHIVE_PLATFORMS.items():
        _audit_phase_exit_native(artifact_dir, platform, target_suffix, issues)
    _audit_windows_bundle(artifact_dir, issues)
    _audit_six_target_bundle(artifact_dir, issues)
    _audit_gates_bundle(artifact_dir, issues)
    return {
        "artifact_dir": str(artifact_dir),
        "mode": "phase-exit",
        "issues": issues,
        "ok": not issues,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("artifact_dir")
    parser.add_argument("--workspace-root", default="")
    parser.add_argument("--json", action="store_true")
    parser.add_argument(
        "--release",
        action="store_true",
        help="audit native opi-sandbox release-archive evidence (SC16-12b) "
        "instead of dogfood run artifacts",
    )
    parser.add_argument(
        "--phase-exit",
        action="store_true",
        help="audit preserved Phase 16 phase-exit evidence (SC16-15b) with "
        "strict rejection of absent/skipped/zero-test/wrong-target/workspace-only "
        "evidence",
    )
    args = parser.parse_args()

    artifact_dir = pathlib.Path(args.artifact_dir)
    workspace_root = (
        pathlib.Path(args.workspace_root)
        if args.workspace_root
        else pathlib.Path.cwd()
    )
    root_forms_norm = _root_forms(args.workspace_root)

    if args.release:
        report = audit_release_evidence(artifact_dir)
        issues = report["issues"]
    elif args.phase_exit:
        report = audit_phase_exit_evidence(artifact_dir)
        issues = report["issues"]
    else:
        ndjson_reports = [
            analyze_ndjson(path, root_forms_norm)
            for path in sorted(artifact_dir.glob("run*.ndjson"))
        ]
        session_reports = [analyze_session(path, root_forms_norm) for path in _session_files(artifact_dir)]
        failure_report = analyze_failure_evidence(artifact_dir)
        commit_references, commit_issues = analyze_commit_references(
            artifact_dir, workspace_root
        )

        issues = []
        for report in ndjson_reports:
            issues.extend(report["issues"])
        for report in session_reports:
            issues.extend(report["issues"])
        issues.extend(failure_report["issues"])
        issues.extend(commit_issues)

        report = {
            "artifact_dir": str(artifact_dir),
            "files": sorted(
                str(path.relative_to(artifact_dir))
                for path in artifact_dir.rglob("*")
                if path.is_file()
            ),
            "ndjson": ndjson_reports,
            "sessions": session_reports,
            "failure_evidence": failure_report,
            "commit_references": commit_references,
            "issues": issues,
            "ok": not issues,
        }

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        status = "PASS" if report["ok"] else "FAIL"
        print(f"artifact audit: {status}")
        for issue in issues:
            location = issue.get("file") or issue.get("platform") or "<artifact>"
            line = issue.get("line")
            suffix = f":{line}" if line else ""
            print(f"- {issue['code']} {location}{suffix}: {issue['message']}")
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
