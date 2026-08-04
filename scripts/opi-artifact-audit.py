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
import subprocess
import sys
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
# Evidence directory layout (one bundle per platform):
#   <dir>/linux/   target, package-lock.toml, extracted/{bin/opi-sandbox,
#                 package.toml}, and a *.txt/*.log smoke evidence file carrying
#                 the `opi-sandbox-smoke: OK` marker.
#   <dir>/macos/   same shape; target is an apple-darwin triple.
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
SMOKE_OK_RE = re.compile(r"opi-sandbox-smoke:\s*OK")
CARGO_PASS_RE = re.compile(r"test result: ok\. ([1-9][0-9]*) passed; 0 failed; 0 ignored")
CARGO_SKIPPED_RE = re.compile(r"test result: ok\. \d+ passed; 0 failed; ([1-9][0-9]*) ignored")
CARGO_ZERO_RE = re.compile(r"test result: ok\. 0 passed; 0 failed; 0 ignored")


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(65536), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _lock_value(lock_text, key):
    match = re.search(
        r'^%s = "([^"]*)"' % re.escape(key), lock_text, re.MULTILINE
    )
    return match.group(1) if match else None


def _bundle_evidence_text(bundle):
    """Concatenate every *.txt / *.log file directly in the bundle dir (not the
    extracted/ subtree) so the pass/skip/zero markers are scanned regardless of
    the exact evidence filename the producer chose."""
    parts = []
    for entry in sorted(bundle.iterdir()):
        if entry.is_file() and entry.suffix.lower() in {".txt", ".log"}:
            parts.append(read_text(entry))
    return "\n".join(parts)


def _classify_evidence(text, platform, issues):
    """Classify smoke/test evidence as pass / skipped / zero-test."""
    if SMOKE_OK_RE.search(text) or CARGO_PASS_RE.search(text):
        return  # passing direct smoke or a non-zero cargo pass.
    if CARGO_SKIPPED_RE.search(text):
        issues.append({
            "code": "skipped_evidence",
            "platform": platform,
            "message": f"{platform} evidence has ignored/skipped tests",
        })
        return
    # No passing marker: zero-test (an explicit 0-passed line) or absent/blank.
    issues.append({
        "code": "zero_test_evidence",
        "platform": platform,
        "message": f"{platform} evidence has no passing smoke/test marker",
    })


def _audit_native_bundle(root, platform, target_suffix, issues):
    bundle = root / platform
    if not bundle.is_dir():
        issues.append({
            "code": "missing_platform_evidence",
            "platform": platform,
            "message": f"missing native evidence bundle for {platform}",
        })
        return
    target = read_text(bundle / "target").strip()
    if not target:
        issues.append({
            "code": "missing_platform_evidence",
            "platform": platform,
            "message": f"{platform} bundle missing target file",
        })
        return
    if "windows" in target or "pc-windows" in target or not target.endswith(target_suffix):
        issues.append({
            "code": "wrong_target_identity",
            "platform": platform,
            "message": f"{platform} target {target} is not a native {platform} triple",
        })
    extracted_bin = bundle / "extracted" / "bin" / "opi-sandbox"
    extracted_manifest = bundle / "extracted" / "package.toml"
    if not extracted_bin.is_file():
        issues.append({
            "code": "workspace_only_binary",
            "platform": platform,
            "message": f"{platform} has no extracted archive binary (workspace-only smoke)",
        })
        return
    if not extracted_manifest.is_file():
        issues.append({
            "code": "workspace_only_binary",
            "platform": platform,
            "message": f"{platform} extracted tree missing package.toml (layout)",
        })
    locked_sha = _lock_value(read_text(bundle / "package-lock.toml"), "executable_sha256")
    actual_sha = sha256_file(extracted_bin)
    if not locked_sha:
        issues.append({
            "code": "provenance_mismatch",
            "platform": platform,
            "message": f"{platform} package-lock.toml missing executable_sha256",
        })
    elif actual_sha != locked_sha:
        issues.append({
            "code": "provenance_mismatch",
            "platform": platform,
            "message": f"{platform} extracted binary sha {actual_sha} != locked {locked_sha}",
        })
    _classify_evidence(_bundle_evidence_text(bundle), platform, issues)


def _audit_windows_bundle(root, issues):
    bundle = root / "windows"
    if not bundle.is_dir():
        issues.append({
            "code": "missing_platform_evidence",
            "platform": "windows",
            "message": "missing windows unsupported-posture evidence bundle",
        })
        return
    if (bundle / "extracted" / "bin" / "opi-sandbox").is_file():
        issues.append({
            "code": "wrong_target_identity",
            "platform": "windows",
            "message": "Windows must not ship an opi-sandbox archive",
        })
    text = _bundle_evidence_text(bundle)
    lower = text.lower()
    if 'supported":false' not in text and "supported = false" not in text and "unsupported" not in lower:
        issues.append({
            "code": "wrong_target_identity",
            "platform": "windows",
            "message": "Windows evidence does not report the unsupported posture",
        })
    _classify_evidence(text, "windows", issues)


def audit_release_evidence(artifact_dir):
    issues = []
    for platform, target_suffix in NATIVE_ARCHIVE_PLATFORMS.items():
        _audit_native_bundle(artifact_dir, platform, target_suffix, issues)
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
    """linux/macos bundle: genuine pass marker; validate archive when present."""
    bundle = root / platform
    if not bundle.is_dir():
        issues.append({
            "code": "missing_platform_evidence",
            "platform": platform,
            "message": f"missing native evidence bundle for {platform}",
        })
        return
    if (bundle / "extracted" / "bin" / "opi-sandbox").is_file():
        # A locally preserved extracted archive: validate target identity,
        # executable-sha provenance, and the smoke/test evidence.
        _audit_native_bundle(root, platform, target_suffix, issues)
        return
    # No local archive (CI-produced): a preserved CI log must carry a genuine
    # pass marker and a provenance note, so absence-of-error is not enough.
    text = _bundle_evidence_text(bundle)
    if not text.strip():
        issues.append({
            "code": "zero_test_evidence",
            "platform": platform,
            "message": f"{platform} has no preserved native smoke/test evidence",
        })
        return
    _classify_evidence(text, platform, issues)
    if not (bundle / "source").is_file():
        issues.append({
            "code": "missing_provenance",
            "platform": platform,
            "message": f"{platform} CI-sourced evidence lacks a `source` provenance note",
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
