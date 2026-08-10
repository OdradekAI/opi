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
import importlib.util
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


def _load_package_helper():
    helper_path = pathlib.Path(__file__).resolve().with_name("opi-sandbox-package.py")
    spec = importlib.util.spec_from_file_location("opi_sandbox_package_helper", helper_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load shared package helper: {helper_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


PACKAGE_HELPER = _load_package_helper()


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


def _record_evidence_filesystem_error(issues, path, error):
    issues.append({
        "code": "evidence_filesystem_error",
        "file": str(path),
        "message": f"cannot read expected evidence file: {error}",
    })


def read_evidence_text(path, issues):
    try:
        return path.read_text(encoding="utf-8", errors="replace")
    except FileNotFoundError:
        return ""
    except OSError as error:
        _record_evidence_filesystem_error(issues, path, error)
        return None


def parse_json_lines(path, issues):
    records = []
    text = read_evidence_text(path, issues)
    if text is None:
        return records
    for line_no, line in enumerate(text.splitlines(), start=1):
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

    for line_no, record in parse_json_lines(path, issues):
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
    records = parse_json_lines(path, issues)
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
    issues = []
    report_blob = "\n".join(
        read_evidence_text(path, issues) or ""
        for path in [artifact_dir / "RUN_SUMMARY.md", artifact_dir / "REVIEW_REPORT.md"]
    )
    report_lower = report_blob.lower()
    mentioned = [w for w in FAILURE_WORDS if w.lower() in report_lower]
    disclosed = any(phrase in report_lower for phrase in DISCLOSURE_PHRASES)

    preserved = []
    for path in artifact_dir.glob("run*.ndjson"):
        body = (read_evidence_text(path, issues) or "").lower()
        preserved.extend(w for w in FAILURE_WORDS if w.lower() in body)
    stderr = (read_evidence_text(artifact_dir / "run.stderr.log", issues) or "").lower()
    preserved.extend(w for w in FAILURE_WORDS if w.lower() in stderr)

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


def collect_commit_references(artifact_dir, issues):
    references = []
    for path in sorted(p for p in artifact_dir.rglob("*") if p.is_file()):
        text = read_evidence_text(path, issues)
        if text is None:
            continue
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
    issues = []
    references = collect_commit_references(artifact_dir, issues)
    for reference in references:
        commit = reference["reference"]
        try:
            lookup = subprocess.run(
                ["git", "-C", str(workspace_root), "cat-file", "-t", commit],
                capture_output=True,
                check=False,
                encoding="utf-8",
                errors="replace",
            )
        except OSError as error:
            issues.append({
                "code": "commit_reference_check_failed",
                **reference,
                "message": f"could not run Git to verify commit reference {commit}: {error}",
            })
            continue
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
    # Deduplicate on the resolved path (so symlinked/reparsed duplicates
    # collapse) but report the as-traversed path. This keeps issue["file"]
    # consistent with the rest of the walker and avoids platform-specific
    # short-name (8.3) expansion surfacing on the report.
    seen = set()
    files = []
    for path in artifact_dir.rglob("sessions*"):
        if path.is_dir():
            for f in path.glob("*.jsonl"):
                resolved = f.resolve()
                if resolved in seen:
                    continue
                seen.add(resolved)
                files.append(f)
    return sorted(files)


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
# Native target bundles are exact: target, package-lock.toml, one native
# archive, and either the synthetic smoke.log fixture or the reviewed smoke/
# output paths produced by opi-sandbox-smoke.sh. Windows permits exactly
# unsupported.log and posture-tests.log. Roots and nested directories are
# lstat/reparse checked and identity-rechecked; regular files are captured once
# with the bounded limits below, while archives use the existing owned on-disk
# snapshot before hashing/extraction.
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
# Allow bounded container metadata/compression overhead beyond the maximum
# extracted payload while keeping the auditor-owned snapshot itself bounded.
ARCHIVE_SNAPSHOT_LIMIT = ARCHIVE_TOTAL_LIMIT + 1024 * 1024
NATIVE_TARGET_SNAPSHOT_LIMIT = 256
NATIVE_LOCK_SNAPSHOT_LIMIT = 16 * 1024
NATIVE_EVIDENCE_FILE_SNAPSHOT_LIMIT = 4 * 1024 * 1024
NATIVE_EVIDENCE_TOTAL_SNAPSHOT_LIMIT = 32 * 1024 * 1024
NATIVE_BUNDLE_ENTRY_LIMIT = 64
WINDOWS_EVIDENCE_FILE_SNAPSHOT_LIMIT = 4 * 1024 * 1024
WINDOWS_EVIDENCE_TOTAL_SNAPSHOT_LIMIT = 8 * 1024 * 1024
WINDOWS_BUNDLE_ENTRY_LIMIT = 8
NATIVE_SMOKE_DIRECTORIES = {
    "smoke",
    "smoke/empty-cwd",
    "smoke/sentinel",
    "smoke/sentinel/opi",
    "smoke/ws",
}
NATIVE_SMOKE_FILES = {
    "smoke/help.txt",
    "smoke/version.txt",
    "smoke/doctor.json",
    "smoke/setup-temp-root-blocker",
    "smoke/setup-stdout.txt",
    "smoke/setup-stderr.txt",
    "smoke/setup-failure-smoke-result.txt",
    "smoke/run-stdout.bin",
    "smoke/run-stderr.bin",
    "smoke/expected-stdout.bin",
    "smoke/expected-stderr.bin",
    "smoke/run-exit.txt",
    "smoke/direct-smoke-result.txt",
    "smoke/filesystem-allow-smoke-result.txt",
    "smoke/filesystem-deny-stdout.txt",
    "smoke/filesystem-deny-stderr.txt",
    "smoke/filesystem-deny-smoke-result.txt",
    "smoke/network-deny-stdout.txt",
    "smoke/network-deny-stderr.txt",
    "smoke/network-deny-smoke-result.txt",
    "smoke/network-allow-stdout.txt",
    "smoke/network-allow-stderr.txt",
    "smoke/network-allow-smoke-result.txt",
    "smoke/backend-smoke-result.txt",
    "smoke/empty-cwd-smoke-result.txt",
    "smoke/smoke-result.txt",
    "smoke/ws/direct-target.sh",
    "smoke/ws/filesystem-allowed.txt",
    "smoke/sentinel/opi/config.toml",
}
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
NATIVE_SENTINEL_SMOKE_RES = {
    name: re.compile(
        rf"opi-sandbox-{re.escape(name)}-smoke:\s*OK\s+archive_sha256=([0-9a-f]{{64}})"
    )
    for name in [
        "empty-cwd",
        "setup-failure",
        "filesystem-allow",
        "filesystem-deny",
        "network-deny",
        "network-allow",
    ]
}
CARGO_PASS_RE = re.compile(r"test result: ok\. ([1-9][0-9]*) passed; 0 failed; 0 ignored")
CARGO_SKIPPED_RE = re.compile(r"test result: ok\. \d+ passed; 0 failed; ([1-9][0-9]*) ignored")
CARGO_ZERO_RE = re.compile(r"test result: ok\. 0 passed; 0 failed; 0 ignored")
NATIVE_SKIP_MARKER_RE = re.compile(r"(?im)^\s*skip\s+outside_write_denied\b")
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


class ArchiveSnapshotError(ValueError):
    def __init__(self, code, message):
        super().__init__(message)
        self.code = code


def sha256_evidence_file(path, issues):
    digest = hashlib.sha256()
    try:
        with open(path, "rb") as handle:
            for chunk in iter(lambda: handle.read(65536), b""):
                digest.update(chunk)
    except OSError as error:
        _record_evidence_filesystem_error(issues, path, error)
        return None
    return digest.hexdigest()


def _copy_archive_snapshot(source, destination):
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = None
    try:
        descriptor = os.open(source, flags)
        with os.fdopen(descriptor, "rb") as input_handle:
            descriptor = None
            opened_stat = os.fstat(input_handle.fileno())
            path_stat = os.lstat(source)
            if (
                not _is_regular_no_reparse(path_stat)
                or not _is_regular_no_reparse(opened_stat)
            ):
                raise ArchiveSnapshotError(
                    "archive_source_not_regular",
                    "archive source is not a regular file",
                )
            if not os.path.samestat(path_stat, opened_stat):
                raise ArchiveSnapshotError(
                    "archive_source_identity_mismatch",
                    "archive source identity changed after open",
                )
            if opened_stat.st_size < 0 or opened_stat.st_size > ARCHIVE_SNAPSHOT_LIMIT:
                raise ArchiveSnapshotError(
                    "archive_snapshot_limit_exceeded",
                    f"archive exceeds snapshot limit of {ARCHIVE_SNAPSHOT_LIMIT} bytes",
                )
            with destination.open("xb") as output_handle:
                copied = 0
                for chunk in iter(lambda: input_handle.read(65536), b""):
                    copied += len(chunk)
                    if copied > ARCHIVE_SNAPSHOT_LIMIT:
                        raise ArchiveSnapshotError(
                            "archive_snapshot_limit_exceeded",
                            f"archive exceeds snapshot limit of {ARCHIVE_SNAPSHOT_LIMIT} bytes while copying",
                        )
                    output_handle.write(chunk)
    finally:
        if descriptor is not None:
            os.close(descriptor)


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
    if NATIVE_SKIP_MARKER_RE.search(text):
        issues.append({
            "code": "skipped_evidence",
            "platform": platform,
            "message": f"{platform} evidence records a skipped outside-write assertion",
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


def _native_evidence_file_policy(relative_name):
    if relative_name == "target":
        return "memory", NATIVE_TARGET_SNAPSHOT_LIMIT
    if relative_name == "package-lock.toml":
        return "memory", NATIVE_LOCK_SNAPSHOT_LIMIT
    if relative_name == "smoke.log" or relative_name in NATIVE_SMOKE_FILES:
        return "memory", NATIVE_EVIDENCE_FILE_SNAPSHOT_LIMIT
    if "/" not in relative_name and _archive_target(pathlib.PurePath(relative_name)):
        return "owned", ARCHIVE_SNAPSHOT_LIMIT
    return None


def _windows_evidence_file_policy(relative_name):
    if relative_name in {"unsupported.log", "posture-tests.log"}:
        return "memory", WINDOWS_EVIDENCE_FILE_SNAPSHOT_LIMIT
    return None


def _snapshot_text_evidence(snapshots):
    return "\n".join(
        snapshot["text"]
        for name, snapshot in sorted(snapshots.items())
        if name not in {"target", "package-lock.toml"}
    )


def _safe_member_name(name):
    if "\\" in name or name.startswith("/"):
        return None
    path = pathlib.PurePosixPath(name)
    if (
        path.is_absolute()
        or not path.parts
        or ".." in path.parts
        or (path.parts and ":" in path.parts[0])
    ):
        return None
    canonical_name = path.as_posix()
    if name != canonical_name:
        return None
    return canonical_name


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
        try:
            major, minor = PACKAGE_HELPER.parse_semver(required["version"])
        except PACKAGE_HELPER.PackageError as error:
            raise ValueError("manifest version is not strict SemVer") from error
        compatible = f">={major}.{minor}.0-0,<{major}.{minor + 1}.0-0"
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


def _parse_lock(text, platform, issues):
    try:
        if text is None:
            raise ValueError("missing package lock snapshot")
        lock = tomllib.loads(text)
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


def _audit_native_smoke(text, platform, archive_sha, issues):
    if not _classify_evidence(text, platform, issues):
        return
    direct = DIRECT_SMOKE_RE.findall(text)
    backend = BACKEND_SMOKE_RE.findall(text)
    named = {
        name: pattern.findall(text)
        for name, pattern in NATIVE_SENTINEL_SMOKE_RES.items()
    }
    missing = [name for name, values in named.items() if not values]
    if not direct or not backend or missing:
        required = ([] if direct else ["direct"])
        required += [] if backend else ["backend"]
        required += missing
        issues.append({
            "code": "missing_smoke_evidence",
            "platform": platform,
            "message": f"{platform} lacks required smoke markers: {', '.join(required)}",
        })
        return
    digests = direct + backend
    for values in named.values():
        digests.extend(values)
    if any(value != archive_sha for value in digests):
        issues.append({
            "code": "archive_digest_mismatch",
            "platform": platform,
            "message": f"{platform} smoke evidence is not bound to archive {archive_sha}",
        })


def _audit_native_bundle(
        root, platform, target_suffix, issues, expected_target=None,
        actual_archive_digests=None):
    bundle = root / platform
    label = platform
    if expected_target is not None:
        bundle = bundle / expected_target
        label = expected_target
    collected = _collect_exact_evidence_bundle(
        bundle,
        label,
        _native_evidence_file_policy,
        NATIVE_SMOKE_DIRECTORIES,
        "missing_platform_evidence",
        f"missing native evidence bundle for {label}",
        NATIVE_BUNDLE_ENTRY_LIMIT,
        NATIVE_EVIDENCE_TOTAL_SNAPSHOT_LIMIT,
        ARCHIVE_SNAPSHOT_LIMIT,
        issues,
    )
    if collected is None:
        return
    snapshots = collected["snapshots"]
    evidence_text = _snapshot_text_evidence(snapshots)
    if "extracted" in collected["observed_entries"]:
        issues.append({
            "code": "caller_prepared_extracted_tree",
            "platform": platform,
            "message": f"{platform} supplies a caller-prepared extracted tree",
        })
    target_snapshot = snapshots.get("target")
    target_file = (target_snapshot["text"] if target_snapshot else "").strip()
    if not target_file:
        issues.append({
            "code": "missing_platform_evidence",
            "platform": platform,
            "message": f"{platform} bundle missing target file",
        })
    archives = list(collected["owned_paths"].values())
    if not archives:
        issues.append({
            "code": "missing_archive",
            "platform": platform,
            "message": f"{platform} bundle has no opi-sandbox archive",
        })
        _classify_evidence(evidence_text, platform, issues)
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

    with tempfile.TemporaryDirectory(prefix="opi-artifact-audit-") as owned:
        owned_root = pathlib.Path(owned)
        snapshot = owned_root / archive.name
        try:
            _copy_archive_snapshot(archive, snapshot)
        except ArchiveSnapshotError as error:
            issues.append({
                "code": error.code,
                "platform": platform,
                "message": f"{platform} archive snapshot is invalid: {error}",
            })
            return
        except OSError as error:
            _record_evidence_filesystem_error(issues, archive, error)
            return
        if not _evidence_bundle_root_unchanged(
                bundle, label, collected["root_stat"], issues):
            return

        archive_sha = sha256_evidence_file(snapshot, issues)
        if archive_sha is None:
            return
        if actual_archive_digests is not None and archive_target in {
                target
                for targets in NATIVE_ARCHIVE_TARGETS.values()
                for target in targets
        }:
            previous = actual_archive_digests.get(archive_target)
            if previous is not None and previous != archive_sha:
                issues.append({
                    "code": "archive_digest_mismatch",
                    "platform": platform,
                    "message": (
                        f"multiple archive snapshots for {archive_target} have different digests"
                    ),
                })
            else:
                # This digest comes from the auditor-owned snapshot that is
                # subsequently extracted and validated. Bound evidence never
                # re-hashes the caller-controlled path.
                actual_archive_digests[archive_target] = archive_sha

        extracted = owned_root / "extracted"
        try:
            _extract_owned_archive(snapshot, extracted)
        except (OSError, tarfile.TarError, zipfile.BadZipFile, ValueError) as error:
            issues.append({
                "code": "invalid_archive_layout",
                "platform": platform,
                "message": f"{platform} archive is invalid: {error}",
            })
            _audit_native_smoke(evidence_text, platform, archive_sha, issues)
            return

        extracted_bin = extracted / "bin" / "opi-sandbox"
        try:
            PACKAGE_HELPER.validate_executable_file(extracted_bin, archive_target)
        except PACKAGE_HELPER.ExecutableFormatError as error:
            issues.append({
                "code": "invalid_executable_format",
                "platform": platform,
                "message": f"{platform} packaged executable is invalid: {error}",
            })
            _audit_native_smoke(evidence_text, platform, archive_sha, issues)
            return
        except PACKAGE_HELPER.ExecutableTargetError as error:
            issues.append({
                "code": "executable_target_mismatch",
                "platform": platform,
                "message": f"{platform} packaged executable target is invalid: {error}",
            })
            _audit_native_smoke(evidence_text, platform, archive_sha, issues)
            return
        except PACKAGE_HELPER.PackageError as error:
            issues.append({
                "code": "invalid_executable_format",
                "platform": platform,
                "message": f"{platform} packaged executable cannot be read: {error}",
            })
            _audit_native_smoke(evidence_text, platform, archive_sha, issues)
            return

        try:
            _validate_archive_assets(extracted)
        except (OSError, ValueError) as error:
            issues.append({
                "code": "invalid_archive_layout",
                "platform": platform,
                "message": f"{platform} archive is invalid: {error}",
            })
            _audit_native_smoke(evidence_text, platform, archive_sha, issues)
            return

        extracted_manifest = extracted / "package.toml"
        manifest_raw, manifest = _parse_manifest(extracted_manifest, platform, issues)
        lock_snapshot = snapshots.get("package-lock.toml")
        lock = _parse_lock(
            lock_snapshot["text"] if lock_snapshot else None, platform, issues
        )
        if manifest is not None and lock is not None:
            manifest_hash = hashlib.sha256(manifest_raw).hexdigest()
            actual_sha = sha256_evidence_file(extracted_bin, issues)
            if actual_sha is None:
                _audit_native_smoke(evidence_text, platform, archive_sha, issues)
                return
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
    if not _evidence_bundle_root_unchanged(
            bundle, label, collected["root_stat"], issues):
        return
    _audit_native_smoke(evidence_text, platform, archive_sha, issues)


def _audit_windows_bundle(root, issues):
    bundle = root / "windows"
    collected = _collect_exact_evidence_bundle(
        bundle,
        "windows",
        _windows_evidence_file_policy,
        set(),
        "missing_platform_evidence",
        "missing windows unsupported-posture evidence bundle",
        WINDOWS_BUNDLE_ENTRY_LIMIT,
        WINDOWS_EVIDENCE_TOTAL_SNAPSHOT_LIMIT,
        0,
        issues,
    )
    if collected is None:
        return
    observed = collected["observed_entries"]
    windows_archives = [
        name for name in observed
        if name.startswith("opi-sandbox-")
        and (name.endswith(".tar.gz") or name.endswith(".zip"))
    ]
    if "extracted" in observed or windows_archives:
        issues.append({
            "code": "wrong_target_identity",
            "platform": "windows",
            "message": "Windows must not ship an opi-sandbox archive",
        })
    snapshots = collected["snapshots"]
    doctor_snapshot = snapshots.get("unsupported.log")
    try:
        doctor = json.loads(doctor_snapshot["text"]) if doctor_snapshot else None
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
    text = _snapshot_text_evidence(snapshots)
    _classify_evidence(text, "windows", issues)


def audit_release_evidence(artifact_dir):
    issues = []
    for platform, target_suffix in NATIVE_ARCHIVE_PLATFORMS.items():
        platform_root = artifact_dir / platform
        targets = NATIVE_ARCHIVE_TARGETS[platform]
        if any(_path_entry_exists_no_follow(platform_root / target) for target in targets):
            for target in targets:
                _audit_native_bundle(
                    artifact_dir, platform, target_suffix, issues, expected_target=target
                )
        else:
            # Keep inspecting legacy flat evidence so defects remain
            # attributable, but it can never satisfy the four-target gate.
            _audit_native_bundle(artifact_dir, platform, target_suffix, issues)
            for target in targets:
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
# layout:
#   windows/    doctor supported=false plus a genuine pass marker (no archive)
#   linux/      the same authenticated native-archive bundles as `--release`
#   macos/      the same authenticated native-archive bundles as `--release`
#   six-target/ one preserved `cargo check --target` log per release triple;
#               each log must record its outcome (a green `Finished` check or an
#               explicit `error[` compiler record), never a blank/absent log
#   gates/      preserved workspace gate evidence (doc guards, product captures,
#               crate-boundary, packaging, release-topology) with a pass marker
#   gates/evidence-identity.json and six-target/evidence-identity.json use the
#               exact schema_version/workflow_run_id/commit_sha/
#               archive_sha256_by_target/files_sha256 schema validated below.
#               The run and commit are explicit audit arguments, never inferred
#               from process environment, and archive digests are compared with
#               the same auditor-owned snapshots used for extraction.
#               Each bundle is flat and exact: no unlisted files, directories,
#               links, or reparse entries. The identity excludes itself from
#               files_sha256. Evidence bytes are opened no-follow, identity-
#               checked, and read once for both digest and parsing, bounded to
#               1 MiB per identity, 16 MiB per evidence file, 64 MiB/64 entries
#               per bundle.
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


def _audit_phase_exit_native(
        root, platform, target_suffix, issues, actual_archive_digests):
    """Phase exit has the same authenticated native-archive requirement."""
    platform_root = root / platform
    targets = NATIVE_ARCHIVE_TARGETS[platform]
    if any(_path_entry_exists_no_follow(platform_root / target) for target in targets):
        for target in targets:
            _audit_native_bundle(
                root,
                platform,
                target_suffix,
                issues,
                expected_target=target,
                actual_archive_digests=actual_archive_digests,
            )
    else:
        _audit_native_bundle(
            root,
            platform,
            target_suffix,
            issues,
            actual_archive_digests=actual_archive_digests,
        )
        for target in targets:
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

EVIDENCE_IDENTITY_FILE = "evidence-identity.json"
EVIDENCE_IDENTITY_FIELDS = {
    "schema_version",
    "workflow_run_id",
    "commit_sha",
    "archive_sha256_by_target",
    "files_sha256",
}
BOUND_ARCHIVE_TARGETS = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]
WORKFLOW_RUN_ID_RE = re.compile(r"^[1-9][0-9]*$")
FULL_COMMIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
EVIDENCE_IDENTITY_SNAPSHOT_LIMIT = 1024 * 1024
EVIDENCE_FILE_SNAPSHOT_LIMIT = 16 * 1024 * 1024
EVIDENCE_BUNDLE_SNAPSHOT_LIMIT = 64 * 1024 * 1024
EVIDENCE_BUNDLE_ENTRY_LIMIT = 64


class EvidenceSnapshotError(ValueError):
    def __init__(self, code, message):
        super().__init__(message)
        self.code = code


def _is_reparse(file_stat):
    attributes = getattr(file_stat, "st_file_attributes", 0) or 0
    reparse_mask = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400) or 0x400
    return bool(attributes & reparse_mask)


def _is_regular_no_reparse(file_stat):
    return stat.S_ISREG(file_stat.st_mode) and not _is_reparse(file_stat)


def _is_directory_no_reparse(file_stat):
    return stat.S_ISDIR(file_stat.st_mode) and not _is_reparse(file_stat)


def _path_entry_exists_no_follow(path):
    try:
        os.lstat(path)
        return True
    except FileNotFoundError:
        return False
    except OSError:
        # Let the exact collector produce the attributable filesystem issue.
        return True


def _read_bounded_evidence_snapshot(path, limit):
    """Read one regular no-follow file once; hash and parse use these bytes."""
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0)
    descriptor = None
    try:
        descriptor = os.open(path, flags)
        with os.fdopen(descriptor, "rb") as handle:
            descriptor = None
            opened_stat = os.fstat(handle.fileno())
            path_stat = os.lstat(path)
            if not _is_regular_no_reparse(opened_stat) or not _is_regular_no_reparse(path_stat):
                raise EvidenceSnapshotError(
                    "invalid_evidence_entry",
                    "evidence entry is not a regular no-follow file",
                )
            if not os.path.samestat(opened_stat, path_stat):
                raise EvidenceSnapshotError(
                    "evidence_snapshot_identity_mismatch",
                    "evidence path identity changed after open",
                )
            if opened_stat.st_size < 0 or opened_stat.st_size > limit:
                raise EvidenceSnapshotError(
                    "evidence_snapshot_limit_exceeded",
                    f"evidence file exceeds the {limit}-byte snapshot limit",
                )

            digest = hashlib.sha256()
            chunks = []
            copied = 0
            while True:
                chunk = handle.read(65536)
                if not chunk:
                    break
                copied += len(chunk)
                if copied > limit:
                    raise EvidenceSnapshotError(
                        "evidence_snapshot_limit_exceeded",
                        f"evidence file exceeds the {limit}-byte snapshot limit while reading",
                    )
                digest.update(chunk)
                chunks.append(chunk)
            if copied != opened_stat.st_size:
                raise EvidenceSnapshotError(
                    "evidence_snapshot_changed",
                    "evidence file size changed while snapshotting",
                )
            raw = b"".join(chunks)
            return {
                "sha256": digest.hexdigest(),
                "text": raw.decode("utf-8", errors="replace"),
                "size": copied,
            }
    finally:
        if descriptor is not None:
            os.close(descriptor)


def _record_evidence_snapshot_error(issues, path, error):
    issues.append({
        "code": error.code,
        "file": str(path),
        "message": str(error),
    })
    if error.code == "invalid_evidence_entry":
        # Preserve the established wrong-kind diagnostic alongside the more
        # precise inventory finding used by bound evidence.
        _record_evidence_filesystem_error(
            issues, path, "expected a regular no-follow evidence file"
        )


def _evidence_bundle_root_unchanged(bundle, label, original_stat, issues):
    try:
        current_stat = os.lstat(bundle)
    except OSError:
        issues.append({
            "code": "evidence_bundle_identity_mismatch",
            "file": str(bundle),
            "message": f"{label} root identity changed during evidence collection",
        })
        return False
    if (
        not _is_directory_no_reparse(current_stat)
        or not os.path.samestat(original_stat, current_stat)
    ):
        issues.append({
            "code": "evidence_bundle_identity_mismatch",
            "file": str(bundle),
            "message": f"{label} root identity changed during evidence collection",
        })
        return False
    return True


def _collect_exact_evidence_bundle(
        bundle, label, file_policy, allowed_directories, missing_code,
        missing_message, entry_limit, memory_aggregate_limit,
        owned_aggregate_limit, issues):
    """Snapshot an exact bundle inventory without following caller entries."""
    try:
        bundle_stat = os.lstat(bundle)
    except FileNotFoundError:
        issues.append({
            "code": missing_code,
            "platform": label,
            "message": missing_message,
        })
        return None
    except OSError as error:
        _record_evidence_filesystem_error(issues, bundle, error)
        return None
    if not _is_directory_no_reparse(bundle_stat):
        issues.append({
            "code": "invalid_evidence_bundle",
            "file": str(bundle),
            "message": f"{label} must be a real non-reparse directory",
        })
        return None

    snapshots = {}
    owned_paths = {}
    observed_entries = set()
    state = {"entries": 0, "memory_bytes": 0, "owned_bytes": 0}

    def unexpected(path, relative_name):
        issues.append({
            "code": "unexpected_evidence_entry",
            "file": str(path),
            "message": f"{label} contains unexpected entry {relative_name!r}",
        })

    def scan_directory(directory, prefix, original_stat):
        try:
            with os.scandir(directory) as iterator:
                entries = [pathlib.Path(entry.path) for entry in iterator]
        except OSError as error:
            _record_evidence_filesystem_error(issues, directory, error)
            return False
        if not _evidence_bundle_root_unchanged(
                directory, f"{label} evidence directory", original_stat, issues):
            return False

        for path in sorted(entries, key=lambda entry: entry.name):
            relative_name = f"{prefix}/{path.name}" if prefix else path.name
            observed_entries.add(relative_name)
            state["entries"] += 1
            if state["entries"] > entry_limit:
                issues.append({
                    "code": "evidence_bundle_entry_limit_exceeded",
                    "file": str(bundle),
                    "message": f"{label} has more than {entry_limit} entries",
                })
                return False
            try:
                path_stat = os.lstat(path)
            except OSError as error:
                _record_evidence_filesystem_error(issues, path, error)
                continue

            policy = file_policy(relative_name)
            if _is_directory_no_reparse(path_stat):
                if policy is not None:
                    error = EvidenceSnapshotError(
                        "invalid_evidence_entry",
                        "expected evidence file is a directory",
                    )
                    _record_evidence_snapshot_error(issues, path, error)
                    continue
                if relative_name not in allowed_directories:
                    unexpected(path, relative_name)
                    continue
                if not scan_directory(path, relative_name, path_stat):
                    return False
                continue
            if not _is_regular_no_reparse(path_stat):
                error = EvidenceSnapshotError(
                    "invalid_evidence_entry",
                    "evidence entry is not a regular no-follow file",
                )
                _record_evidence_snapshot_error(issues, path, error)
                continue
            if policy is None:
                unexpected(path, relative_name)
                continue

            ownership, limit = policy
            if path_stat.st_size < 0 or path_stat.st_size > limit:
                code = (
                    "archive_snapshot_limit_exceeded"
                    if ownership == "owned"
                    else "evidence_snapshot_limit_exceeded"
                )
                issues.append({
                    "code": code,
                    "file": str(path),
                    "message": f"{relative_name} exceeds its {limit}-byte snapshot limit",
                })
                continue
            aggregate_key = (
                "owned_bytes" if ownership == "owned" else "memory_bytes"
            )
            aggregate_limit = (
                owned_aggregate_limit
                if ownership == "owned"
                else memory_aggregate_limit
            )
            if state[aggregate_key] + path_stat.st_size > aggregate_limit:
                issues.append({
                    "code": "evidence_bundle_snapshot_limit_exceeded",
                    "file": str(bundle),
                    "message": f"{label} exceeds its {aggregate_limit}-byte snapshot budget",
                })
                continue

            if ownership == "owned":
                owned_paths[relative_name] = path
                state[aggregate_key] += path_stat.st_size
                continue
            try:
                snapshot = _read_bounded_evidence_snapshot(path, limit)
            except EvidenceSnapshotError as error:
                _record_evidence_snapshot_error(issues, path, error)
                continue
            except OSError as error:
                _record_evidence_filesystem_error(issues, path, error)
                continue
            snapshots[relative_name] = snapshot
            state[aggregate_key] += snapshot["size"]

        return _evidence_bundle_root_unchanged(
            directory, f"{label} evidence directory", original_stat, issues
        )

    if not scan_directory(bundle, "", bundle_stat):
        return None
    return {
        "root_stat": bundle_stat,
        "snapshots": snapshots,
        "owned_paths": owned_paths,
        "observed_entries": observed_entries,
    }


def _collect_bound_evidence_bundle(
        bundle, label, allowed_name, missing_code, missing_message, issues):
    try:
        bundle_stat = os.lstat(bundle)
    except FileNotFoundError:
        issues.append({"code": missing_code, "message": missing_message})
        return None, {}
    except OSError as error:
        _record_evidence_filesystem_error(issues, bundle, error)
        return None, {}
    if not _is_directory_no_reparse(bundle_stat):
        issues.append({
            "code": "invalid_evidence_bundle",
            "file": str(bundle),
            "message": f"{label} must be a real non-reparse directory",
        })
        return None, {}

    entries = []
    try:
        with os.scandir(bundle) as iterator:
            for entry in iterator:
                entries.append(pathlib.Path(entry.path))
                if len(entries) > EVIDENCE_BUNDLE_ENTRY_LIMIT:
                    issues.append({
                        "code": "evidence_bundle_entry_limit_exceeded",
                        "file": str(bundle),
                        "message": (
                            f"{label} has more than {EVIDENCE_BUNDLE_ENTRY_LIMIT} entries"
                        ),
                    })
                    return None, {}
    except OSError as error:
        _record_evidence_filesystem_error(issues, bundle, error)
        return None, {}
    if not _evidence_bundle_root_unchanged(bundle, label, bundle_stat, issues):
        return None, {}

    identity_snapshot = None
    evidence_snapshots = {}
    total_size = 0
    for path in sorted(entries, key=lambda entry: entry.name):
        name = path.name
        try:
            path_stat = os.lstat(path)
        except OSError as error:
            _record_evidence_filesystem_error(issues, path, error)
            continue
        if not _is_regular_no_reparse(path_stat):
            error = EvidenceSnapshotError(
                "invalid_evidence_entry",
                "nested directories, links, and non-regular evidence entries are forbidden",
            )
            _record_evidence_snapshot_error(issues, path, error)
            continue
        if name != EVIDENCE_IDENTITY_FILE and not allowed_name(name):
            issues.append({
                "code": "unexpected_evidence_entry",
                "file": str(path),
                "message": f"{label} contains an entry outside its exact inventory",
            })
            continue

        limit = (
            EVIDENCE_IDENTITY_SNAPSHOT_LIMIT
            if name == EVIDENCE_IDENTITY_FILE
            else EVIDENCE_FILE_SNAPSHOT_LIMIT
        )
        try:
            snapshot = _read_bounded_evidence_snapshot(path, limit)
        except EvidenceSnapshotError as error:
            _record_evidence_snapshot_error(issues, path, error)
            continue
        except OSError as error:
            _record_evidence_filesystem_error(issues, path, error)
            continue
        if total_size + snapshot["size"] > EVIDENCE_BUNDLE_SNAPSHOT_LIMIT:
            issues.append({
                "code": "evidence_bundle_snapshot_limit_exceeded",
                "file": str(bundle),
                "message": (
                    f"{label} exceeds the {EVIDENCE_BUNDLE_SNAPSHOT_LIMIT}-byte snapshot budget"
                ),
            })
            continue
        total_size += snapshot["size"]
        if name == EVIDENCE_IDENTITY_FILE:
            identity_snapshot = snapshot
        else:
            evidence_snapshots[name] = snapshot
    if not _evidence_bundle_root_unchanged(bundle, label, bundle_stat, issues):
        return None, {}
    return identity_snapshot, evidence_snapshots


def _invalid_evidence_identity(issues, label, message):
    issues.append({
        "code": "invalid_evidence_identity",
        "file": f"{label}/{EVIDENCE_IDENTITY_FILE}",
        "message": message,
    })


def _read_bound_evidence_identity(
        bundle, label, identity_snapshot, evidence_snapshots, expected_run_id,
        expected_commit_sha, actual_archive_digests, issues):
    identity_path = bundle / EVIDENCE_IDENTITY_FILE
    if identity_snapshot is None:
        issues.append({
            "code": "missing_evidence_identity",
            "file": str(identity_path),
            "message": f"{label} lacks the required structured evidence identity",
        })
        return None
    try:
        identity = json.loads(identity_snapshot["text"])
    except (json.JSONDecodeError, TypeError):
        _invalid_evidence_identity(issues, label, "identity must be valid JSON")
        return None
    if not isinstance(identity, dict) or set(identity) != EVIDENCE_IDENTITY_FIELDS:
        _invalid_evidence_identity(
            issues,
            label,
            "identity must contain exactly the five version, run, commit, archive, and file fields",
        )
        return None
    schema_version = identity.get("schema_version")
    if type(schema_version) is not int or schema_version != 1:
        _invalid_evidence_identity(
            issues, label, "schema_version must be the JSON integer 1"
        )
        return None

    run_id = identity.get("workflow_run_id")
    commit_sha = identity.get("commit_sha")
    archives = identity.get("archive_sha256_by_target")
    files = identity.get("files_sha256")
    if not isinstance(run_id, str) or not WORKFLOW_RUN_ID_RE.fullmatch(run_id):
        _invalid_evidence_identity(
            issues, label, "workflow_run_id must be a non-zero decimal string"
        )
        return None
    if not isinstance(commit_sha, str) or not FULL_COMMIT_SHA_RE.fullmatch(commit_sha):
        _invalid_evidence_identity(
            issues, label, "commit_sha must be a full lowercase 40-hex Git SHA"
        )
        return None
    if (
        not isinstance(archives, dict)
        or set(archives) != set(BOUND_ARCHIVE_TARGETS)
        or not all(
            isinstance(value, str) and SHA256_RE.fullmatch(value)
            for value in archives.values()
        )
    ):
        _invalid_evidence_identity(
            issues,
            label,
            "archive_sha256_by_target must contain exactly the four native targets with lowercase SHA-256 values",
        )
        return None

    expected_file_names = set(evidence_snapshots)
    if (
        not isinstance(files, dict)
        or set(files) != expected_file_names
        or not all(
            isinstance(value, str) and SHA256_RE.fullmatch(value)
            for value in files.values()
        )
    ):
        _invalid_evidence_identity(
            issues,
            label,
            "files_sha256 must bind every and only the evidence files in this bundle",
        )
        return None

    if expected_run_id is not None and run_id != expected_run_id:
        issues.append({
            "code": "run_identity_mismatch",
            "file": str(identity_path),
            "message": f"{label} workflow run identity does not match the audit invocation",
        })
    if expected_commit_sha is not None and commit_sha != expected_commit_sha:
        issues.append({
            "code": "commit_identity_mismatch",
            "file": str(identity_path),
            "message": f"{label} commit identity does not match the audit invocation",
        })

    for target in BOUND_ARCHIVE_TARGETS:
        if archives[target] != actual_archive_digests.get(target):
            issues.append({
                "code": "archive_digest_mismatch",
                "file": str(identity_path),
                "message": (
                    f"{label} archive digest for {target} does not match "
                    "the auditor-owned archive snapshot"
                ),
            })

    for name, snapshot in evidence_snapshots.items():
        if files[name] != snapshot["sha256"]:
            path = bundle / name
            issues.append({
                "code": "evidence_file_digest_mismatch",
                "file": str(path),
                "message": f"{label} evidence file is not bound to its declared digest",
            })

    return {
        "workflow_run_id": run_id,
        "commit_sha": commit_sha,
        "archive_sha256_by_target": archives,
    }


def _compare_bound_identities(gates_identity, six_target_identity, issues):
    if (
        gates_identity is not None
        and six_target_identity is not None
        and gates_identity != six_target_identity
    ):
        issues.append({
            "code": "evidence_identity_mismatch",
            "message": (
                "gates and six-target evidence were not produced by the same "
                "workflow run, commit, and native archive set"
            ),
        })


def _gate_pass_marker(category, text):
    if GATE_PASS_RE.search(text):
        return True
    if category in GATE_TEST_CATEGORIES:
        # A test-based gate must prove a non-zero pass; 0-passed/Finished-only
        # evidence is zero-test.
        return False
    return bool(GATE_CLEAN_RE.search(text))


def _audit_six_target_bundle(
        root, expected_run_id, expected_commit_sha, actual_archive_digests, issues):
    six = root / "six-target"
    identity_snapshot, evidence_snapshots = _collect_bound_evidence_bundle(
        six,
        "six-target",
        lambda name: name == "source" or pathlib.PurePath(name).suffix.lower() in {".txt", ".log"},
        "missing_six_target_evidence",
        "missing six-target evidence bundle",
        issues,
    )
    logs = {
        name: snapshot["text"]
        for name, snapshot in evidence_snapshots.items()
        if pathlib.PurePath(name).suffix.lower() in {".txt", ".log"}
    }
    if not logs:
        issues.append({
            "code": "zero_test_evidence",
            "message": "six-target bundle has no preserved logs",
        })
    if "source" not in evidence_snapshots:
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
    return _read_bound_evidence_identity(
        six,
        "six-target",
        identity_snapshot,
        evidence_snapshots,
        expected_run_id,
        expected_commit_sha,
        actual_archive_digests,
        issues,
    )


def _audit_gates_bundle(
        root, expected_run_id, expected_commit_sha, actual_archive_digests, issues):
    gates = root / "gates"
    identity_snapshot, evidence_snapshots = _collect_bound_evidence_bundle(
        gates,
        "gates",
        lambda name: pathlib.PurePath(name).suffix.lower() in {".txt", ".log"},
        "missing_gate_evidence",
        "missing workspace gate evidence bundle",
        issues,
    )
    by_name = {
        name: snapshot["text"]
        for name, snapshot in evidence_snapshots.items()
    }
    if not by_name:
        issues.append({
            "code": "zero_test_evidence",
            "message": "gates bundle has no preserved pass-marked captures",
        })
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
            if category in GATE_TEST_CATEGORIES and CARGO_SKIPPED_RE.search(text):
                issues.append({
                    "code": "skipped_evidence",
                    "message": f"gates/{name} has ignored/skipped tests for `{category}`",
                })
                continue
            if not _gate_pass_marker(category, text):
                issues.append({
                    "code": "zero_test_evidence",
                    "message": f"gates/{name} lacks a genuine pass marker for `{category}`",
                })
    return _read_bound_evidence_identity(
        gates,
        "gates",
        identity_snapshot,
        evidence_snapshots,
        expected_run_id,
        expected_commit_sha,
        actual_archive_digests,
        issues,
    )


def audit_phase_exit_evidence(
        artifact_dir, expected_run_id=None, expected_commit_sha=None):
    issues = []
    if expected_run_id is None or expected_commit_sha is None:
        issues.append({
            "code": "missing_declared_identity",
            "message": (
                "phase-exit audit requires explicit --workflow-run-id and --commit-sha values"
            ),
        })
    if expected_run_id is not None and not WORKFLOW_RUN_ID_RE.fullmatch(expected_run_id):
        issues.append({
            "code": "invalid_declared_identity",
            "message": "declared workflow run id must be a non-zero decimal string",
        })
        expected_run_id = None
    if expected_commit_sha is not None and not FULL_COMMIT_SHA_RE.fullmatch(expected_commit_sha):
        issues.append({
            "code": "invalid_declared_identity",
            "message": "declared commit must be a full lowercase 40-hex Git SHA",
        })
        expected_commit_sha = None
    actual_archive_digests = {}
    for platform, target_suffix in NATIVE_ARCHIVE_PLATFORMS.items():
        _audit_phase_exit_native(
            artifact_dir,
            platform,
            target_suffix,
            issues,
            actual_archive_digests,
        )
    _audit_windows_bundle(artifact_dir, issues)
    six_target_identity = _audit_six_target_bundle(
        artifact_dir,
        expected_run_id,
        expected_commit_sha,
        actual_archive_digests,
        issues,
    )
    gates_identity = _audit_gates_bundle(
        artifact_dir,
        expected_run_id,
        expected_commit_sha,
        actual_archive_digests,
        issues,
    )
    _compare_bound_identities(gates_identity, six_target_identity, issues)
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
        "--workflow-run-id",
        help="explicit workflow run identity for bound phase-exit evidence",
    )
    parser.add_argument(
        "--commit-sha",
        help="explicit full commit SHA for bound phase-exit evidence",
    )
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
        report = audit_phase_exit_evidence(
            artifact_dir,
            expected_run_id=args.workflow_run_id,
            expected_commit_sha=args.commit_sha,
        )
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
