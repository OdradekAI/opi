#!/usr/bin/env python3
"""Phase 18 baseline capture for the Opi Minimal Runtime.

Binds a pre-phase18 behavior snapshot of the Reference Product to an exact
start commit and tree, before any Phase 18 Cargo, Rust, product-configuration,
or package-registration change. The evidence directory records, for every
captured behavior family:

* the exact argv, cwd, relevant environment projection, exit status, and
  classification;
* raw stdout/stderr files with SHA-256 digests;
* an isolated temp-root before/after manifest and a post-check child-process
  census proving no background activity remains;
* a source-derived dependency/call-site inventory and per-check fixture
  digests, all read from the bound commit tree; and
* an index referencing every raw file and the receipt.

The directory must pass ``scripts/opi-artifact-audit.py``. ``--verify``
re-checks the persisted receipt, digests, inventory, and audit without
rerunning the historical runtime.

Capture mode is Linux-bound (the process census reads ``/proc``) and requires
``CARGO_TARGET_DIR`` so no per-session target directory is created. Verify
mode is platform-neutral.

Usage::

    python scripts/capture-phase18-minimal-runtime-baseline.py \
        --expected-commit <sha> \
        --output crates/opi-eval/tests/fixtures/minimal-runtime/pre-phase18 \
        --require-clean-product-paths

    python scripts/capture-phase18-minimal-runtime-baseline.py \
        --verify --expected-commit <sha> \
        --input crates/opi-eval/tests/fixtures/minimal-runtime/pre-phase18
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import io
import json
import os
import re
import signal
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path

RECEIPT_SCHEMA = "phase18-minimal-runtime-baseline-receipt/1"
DEFAULT_TIMEOUT_SECONDS = 1800
LEDGER_PATH = ".opi-impl-state.json"
AUDIT_SCRIPT = "scripts/opi-artifact-audit.py"

# Task-owned non-product paths that may differ from the bound commit while the
# capture runs: the helper itself, its test, and the harness ledger.
DEFAULT_WHITELIST = frozenset(
    {
        "scripts/capture-phase18-minimal-runtime-baseline.py",
        "scripts/test_capture_phase18_minimal_runtime_baseline.py",
        LEDGER_PATH,
    }
)

ENV_PROJECTION_FULL = (
    "CARGO_TARGET_DIR",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "OPI_CARGO_CACHE_ROOT",
    "TMPDIR",
)
ENV_PROJECTION_NAME_ONLY = ("CI", "HOME")

# Source anchors whose tree-bound digests make later Minimal Runtime drift
# detectable without rerunning the captured commands.
SOURCE_ANCHORS = (
    "crates/opi-coding-agent/src/cli.rs",
    "crates/opi-coding-agent/src/main.rs",
    "crates/opi-coding-agent/src/runner.rs",
    "crates/opi-coding-agent/src/execution/runtime.rs",
    "crates/opi-coding-agent/src/execution/router.rs",
)

COMPANION_REFERENCE_RE = re.compile(r"opi[-_]eval")


class CaptureRejected(Exception):
    """Raised when capture or verification must fail closed."""


class Check:
    __slots__ = ("family", "argv", "fixture")

    def __init__(self, family: str, argv: list[str], fixture: str) -> None:
        self.family = family
        self.argv = argv
        self.fixture = fixture


CAPTURED_CHECKS: list[Check] = [
    Check(
        "ordinary-cli-io",
        ["cargo", "run", "-p", "opi-coding-agent", "--bin", "opi", "--", "--help"],
        "crates/opi-coding-agent/src/cli.rs",
    ),
    Check(
        "minimal-runtime-evidence",
        ["cargo", "test", "-p", "opi-coding-agent", "--test", "execution_minimal_runtime"],
        "crates/opi-coding-agent/tests/execution_minimal_runtime.rs",
    ),
    Check(
        "minimal-runtime-evidence",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "execution_runtime",
            "minimal_runtime",
        ],
        "crates/opi-coding-agent/tests/execution_runtime.rs",
    ),
    Check(
        "user-policy",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "execution_routing",
            "production_minimal_runtime_local_deny_omits_bash_and_surfaces_stable_code",
        ],
        "crates/opi-coding-agent/tests/execution_routing.rs",
    ),
    Check(
        "minimal-runtime-evidence",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "phase17_product_evidence",
            "phase17_default_harness_emits_no_evidence",
        ],
        "crates/opi-coding-agent/tests/phase17_product_evidence.rs",
    ),
    Check(
        "minimal-runtime-evidence",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "phase17_cross_mode",
            "phase17_all_public_product_modes_share_runtime_semantics",
        ],
        "crates/opi-coding-agent/tests/phase17_cross_mode.rs",
    ),
    Check(
        "user-policy",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "phase17_tool_authority",
            "phase17_model_content_cannot_expand_effective_policy",
        ],
        "crates/opi-coding-agent/tests/phase17_tool_authority.rs",
    ),
    Check(
        "provider-routing",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "non_interactive",
            "runner_text_prompt_stdout_exit0",
        ],
        "crates/opi-coding-agent/tests/non_interactive.rs",
    ),
    Check(
        "provider-routing",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "non_interactive",
            "runner_provider_error_stderr_exit4",
        ],
        "crates/opi-coding-agent/tests/non_interactive.rs",
    ),
    Check(
        "tool-behavior",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "non_interactive",
            "runner_readonly_tool_succeeds",
        ],
        "crates/opi-coding-agent/tests/non_interactive.rs",
    ),
    Check(
        "session-persistence",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "non_interactive",
            "runner_resume_forwards_branch_summary_to_provider",
        ],
        "crates/opi-coding-agent/tests/non_interactive.rs",
    ),
    Check(
        "session-persistence",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "session_runtime",
            "multi_turn_session_persistence",
        ],
        "crates/opi-coding-agent/tests/session_runtime.rs",
    ),
    Check(
        "background-process-cleanup",
        [
            "cargo",
            "test",
            "-p",
            "opi-coding-agent",
            "--test",
            "sandbox_l0",
            "clean_exit_kills_surviving_background_descendants",
        ],
        "crates/opi-coding-agent/tests/sandbox_l0.rs",
    ),
]

REQUIRED_FAMILIES = tuple(
    sorted({check.family for check in CAPTURED_CHECKS})
)


# ---------------------------------------------------------------------------
# Small shared helpers
# ---------------------------------------------------------------------------


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: Path, value) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="\n") as handle:
        json.dump(value, handle, indent=2, ensure_ascii=False, sort_keys=True)
        handle.write("\n")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def normalize_newlines(data: bytes) -> bytes:
    return data.replace(b"\r\n", b"\n")


def utc_now() -> str:
    return datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat()


def git(repo: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        raise CaptureRejected(
            f"git {' '.join(args)} failed: {result.stderr.strip()}"
        )
    return result.stdout


def git_blob(repo: Path, revision: str, path: str) -> bytes | None:
    result = subprocess.run(
        ["git", "-C", str(repo), "show", f"{revision}:{path}"],
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    return result.stdout


def commit_tree(repo: Path, commit: str) -> str:
    return git(repo, "rev-parse", f"{commit}^{{tree}}").strip()


def _reject_late(receipt_path: Path | None, reason: str) -> None:
    if receipt_path is not None:
        write_json(
            receipt_path,
            {
                "schema": RECEIPT_SCHEMA,
                "status": "rejected",
                "reject_reason": reason,
            },
        )
    raise CaptureRejected(reason)


# ---------------------------------------------------------------------------
# Product-path guard
# ---------------------------------------------------------------------------


def changed_product_paths(
    repo: Path, expected_commit: str, whitelist: frozenset[str]
) -> set[str]:
    """Tracked paths whose content differs semantically from the commit.

    A path is clean when it is whitelisted or when its on-disk bytes differ
    from the commit only by CRLF/LF line endings.
    """
    diff = git(repo, "diff", "--name-only", expected_commit).splitlines()
    changed: set[str] = set()
    for rel in sorted({line.strip() for line in diff if line.strip()}):
        if rel in whitelist:
            continue
        committed = git_blob(repo, expected_commit, rel)
        disk_path = repo / rel
        if not disk_path.is_file():
            changed.add(rel)
            continue
        if committed is None:
            changed.add(rel)
            continue
        if normalize_newlines(committed) != normalize_newlines(
            disk_path.read_bytes()
        ):
            changed.add(rel)
    return changed


def unexpected_untracked(
    repo: Path, output_dir: str, whitelist: frozenset[str] = frozenset()
) -> list[str]:
    others = git(
        repo, "ls-files", "--others", "--exclude-standard"
    ).splitlines()
    prefix = output_dir.replace("\\", "/").strip("/") + "/"
    return sorted(
        line.strip()
        for line in others
        if line.strip()
        and not line.strip().startswith(prefix)
        and line.strip() not in whitelist
    )


# ---------------------------------------------------------------------------
# Process census
# ---------------------------------------------------------------------------


def census(temp_root: Path) -> list[dict]:
    """Residual processes parented to us or living inside the temp root.

    Linux-only by design: the capture environment is pinned to Linux and a
    missing ``/proc`` fails closed.
    """
    if not sys.platform.startswith("linux"):
        raise CaptureRejected(
            f"process census requires /proc (Linux); unsupported on {sys.platform}"
        )
    root = str(temp_root.resolve())
    found: list[dict] = []
    try:
        entries = os.listdir("/proc")
    except OSError as error:
        raise CaptureRejected(f"cannot read /proc for census: {error}") from error
    for entry in entries:
        if not entry.isdigit():
            continue
        stat_path = f"/proc/{entry}/stat"
        try:
            with open(stat_path, "rb") as handle:
                stat_line = handle.read()
        except OSError:
            continue
        close = stat_line.rfind(b")")
        if close < 0:
            continue
        comm = stat_line[stat_line.find(b"(") + 1 : close].decode(
            "utf-8", "replace"
        )
        fields = stat_line[close + 2 :].split()
        if len(fields) < 2:
            continue
        try:
            ppid = int(fields[1])
        except ValueError:
            continue
        cwd: str | None = None
        under_root = False
        try:
            cwd = os.readlink(f"/proc/{entry}/cwd")
            under_root = cwd.startswith(root + os.sep) or cwd == root
        except OSError:
            pass
        if ppid == os.getpid() or under_root:
            found.append(
                {
                    "pid": int(entry),
                    "ppid": ppid,
                    "comm": comm,
                    "cwd": cwd,
                    "cwd_under_temp_root": under_root,
                }
            )
    return sorted(found, key=lambda item: item["pid"])


# ---------------------------------------------------------------------------
# Source-derived dependency/call-site inventory
# ---------------------------------------------------------------------------


def _read_tree(repo: Path, commit: str) -> dict[str, bytes]:
    """Materialize the commit tree through `git archive` without a checkout."""
    proc = subprocess.run(
        ["git", "-C", str(repo), "archive", "--format=tar", commit],
        capture_output=True,
        check=False,
    )
    if proc.returncode != 0:
        raise CaptureRejected(
            f"git archive {commit} failed: {proc.stderr.decode('utf-8', 'replace')}"
        )
    files: dict[str, bytes] = {}
    with tarfile.open(fileobj=io.BytesIO(proc.stdout), mode="r:") as archive:
        for member in archive.getmembers():
            if not member.isfile():
                continue
            handle = archive.extractfile(member)
            if handle is not None:
                files[member.name] = handle.read()
    return files


def _parse_manifest_dependencies(manifest_text: str) -> dict[str, list[str]]:
    import tomllib

    manifest = tomllib.loads(manifest_text)
    tables: dict[str, list[str]] = {}

    def record(table: str, deps: dict, *, always: bool = False) -> None:
        names = sorted(
            (
                value.get("package", key)
                if isinstance(value, dict)
                else key
            )
            for key, value in deps.items()
        )
        if names or always:
            tables[table] = names

    record("dependencies", manifest.get("dependencies", {}), always=True)
    record("dev-dependencies", manifest.get("dev-dependencies", {}), always=True)
    record("build-dependencies", manifest.get("build-dependencies", {}), always=True)
    for target, tables_by_kind in manifest.get("target", {}).items():
        if not isinstance(tables_by_kind, dict):
            continue
        for kind, deps in tables_by_kind.items():
            if isinstance(deps, dict):
                record(f"target.{target}.{kind}", deps)
    return tables


def derive_source_inventory(repo: Path, commit: str) -> dict:
    """Dependency and call-site inventory derived only from the commit tree."""
    tree = _read_tree(repo, commit)
    root_manifest = tree.get("Cargo.toml")
    if root_manifest is None:
        raise CaptureRejected(f"{commit}: root Cargo.toml missing from tree")

    import tomllib

    members = tomllib.loads(root_manifest.decode("utf-8")).get("workspace", {}).get(
        "members", []
    )
    member_inventory: dict[str, dict] = {}
    for member in sorted(members):
        manifest_path = f"{member}/Cargo.toml"
        manifest_text = tree.get(manifest_path)
        entry: dict = {"manifest": manifest_path}
        if manifest_text is None:
            entry["missing"] = True
        else:
            entry["dependency_tables"] = _parse_manifest_dependencies(
                manifest_text.decode("utf-8")
            )
        member_inventory[member] = entry

    companion_paths: list[str] = []
    occurrences = 0
    for name in sorted(tree):
        if not name.endswith((".rs", ".toml", ".md", ".yml", ".yaml", ".py", ".sh")):
            continue
        count = len(COMPANION_REFERENCE_RE.findall(tree[name].decode("utf-8", "replace")))
        if count:
            companion_paths.append(name)
            occurrences += count

    anchor_digests = {
        anchor: sha256_bytes(tree[anchor]) for anchor in SOURCE_ANCHORS if anchor in tree
    }

    return {
        "commit": commit,
        "members": member_inventory,
        "companion_references": {
            "pattern": COMPANION_REFERENCE_RE.pattern,
            "count": occurrences,
            "paths": companion_paths,
        },
        "anchor_digests": anchor_digests,
    }


def source_inventory_digest(inventory: dict) -> str:
    return sha256_bytes(
        json.dumps(inventory, sort_keys=True, ensure_ascii=False).encode("utf-8")
    )


# ---------------------------------------------------------------------------
# Environment projection
# ---------------------------------------------------------------------------


def environment_projection(extra_env: dict[str, str]) -> dict:
    merged = dict(os.environ)
    merged.update(extra_env)
    projection: dict = {}
    for key in ENV_PROJECTION_FULL:
        projection[key] = merged.get(key)
    for key in ENV_PROJECTION_NAME_ONLY:
        projection[f"{key}_present"] = merged.get(key) is not None
    cargo = _which_in(merged, "cargo")
    rustc = _which_in(merged, "rustc")
    projection["cargo_path"] = cargo
    projection["rustc_path"] = rustc
    related = sorted(
        key
        for key in merged
        if (key.startswith(("CARGO_", "RUST", "OPI_")) and key not in projection)
    )
    projection["related_names_only"] = related
    return projection


def _which_in(env: dict[str, str], tool: str) -> str | None:
    import shutil

    # Keep the PATH-resolved shim path verbatim: rustup tool shims are links to
    # the rustup binary and dispatch on their own name, so resolving them to a
    # real path would report the wrong tool.
    return shutil.which(tool, path=env.get("PATH"))


def tool_versions(env: dict[str, str]) -> dict:
    versions = {}
    for tool in ("cargo", "rustc"):
        path = _which_in(env, tool)
        if path is None:
            versions[tool] = None
            continue
        result = subprocess.run(
            [path, "--version"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            check=False,
        )
        versions[tool] = (
            result.stdout.strip() if result.returncode == 0 else None
        )
    return versions


# ---------------------------------------------------------------------------
# Capture
# ---------------------------------------------------------------------------


def _default_audit_runner() -> object:
    def run(repo: Path, artifact_dir: Path) -> tuple[int, str, str]:
        argv = [
            sys.executable,
            str(Path(__file__).resolve().parent / "opi-artifact-audit.py"),
            str(artifact_dir),
            "--workspace-root",
            str(repo),
            "--json",
        ]
        result = subprocess.run(
            argv, capture_output=True, text=True, encoding="utf-8", errors="replace"
        )
        return result.returncode, result.stdout, result.stderr

    return run


def _run_check(
    check: Check,
    check_id: str,
    repo: Path,
    env: dict[str, str],
    out_dir: Path,
    timeout: int,
) -> dict:
    check_dir = out_dir / "checks" / f"{check_id}-{check.family}"
    check_dir.mkdir(parents=True, exist_ok=True)
    stdout_path = check_dir / "stdout.log"
    stderr_path = check_dir / "stderr.log"

    with open(stdout_path, "wb") as out_handle, open(stderr_path, "wb") as err_handle:
        proc = subprocess.Popen(
            list(check.argv),
            cwd=str(repo),
            env=env,
            start_new_session=True,
            stdout=out_handle,
            stderr=err_handle,
        )
        timed_out = False
        try:
            returncode = proc.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out = True
            try:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
            except (ProcessLookupError, PermissionError):
                proc.kill()
            returncode = proc.wait()

    temp_root = Path(env["TMPDIR"])
    residual = census(temp_root)
    return {
        "family": check.family,
        "argv": list(check.argv),
        "cwd": str(repo),
        "exit": returncode,
        "timed_out": timed_out,
        "stdout_file": str(stdout_path.relative_to(out_dir)).replace("\\", "/"),
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_file": str(stderr_path.relative_to(out_dir)).replace("\\", "/"),
        "stderr_sha256": sha256_file(stderr_path),
        "census": {"residual_processes": residual},
    }


def _manifest_tree(temp_root: Path) -> list[dict]:
    entries: list[dict] = []
    if not temp_root.exists():
        return entries
    for path in sorted(temp_root.rglob("*")):
        if path.is_file():
            entries.append(
                {
                    "path": str(path.relative_to(temp_root)).replace("\\", "/"),
                    "bytes": path.stat().st_size,
                    "sha256": sha256_file(path),
                }
            )
    return entries


def capture(
    repo: Path,
    expected_commit: str,
    output_dir: Path,
    *,
    checks: list[Check] | None = None,
    helper_path: Path | None = None,
    test_path: Path | None = None,
    audit_runner=None,
    timeout: int = DEFAULT_TIMEOUT_SECONDS,
    extra_env: dict[str, str] | None = None,
    whitelist: frozenset[str] = DEFAULT_WHITELIST,
) -> dict:
    """Run the bound baseline capture and write the evidence directory."""
    if checks is None:
        checks = CAPTURED_CHECKS
    if helper_path is None:
        helper_path = Path(__file__).resolve()
    if test_path is None:
        test_path = (
            Path(__file__).resolve().parent
            / "test_capture_phase18_minimal_runtime_baseline.py"
        )
    if audit_runner is None:
        audit_runner = _default_audit_runner()

    output_rel = str(output_dir).replace("\\", "/")
    if str(output_dir.resolve()).startswith(str(repo.resolve())):
        output_rel = str(output_dir.resolve().relative_to(repo.resolve())).replace(
            "\\", "/"
        )

    receipt_path = output_dir / "receipt.json"

    # Fail closed on head mismatch before touching the tree.
    head = git(repo, "rev-parse", "HEAD").strip()
    if head != expected_commit:
        _reject_late(
            receipt_path,
            f"commit mismatch: HEAD {head} != expected {expected_commit}",
        )

    families = {check.family for check in checks}
    missing = sorted(set(REQUIRED_FAMILIES) - families)
    if missing:
        _reject_late(
            receipt_path,
            f"missing behavior family: {', '.join(missing)}",
        )

    changed = changed_product_paths(repo, expected_commit, whitelist)
    if changed:
        _reject_late(
            receipt_path,
            f"dirty product path (late capture or drift): {', '.join(sorted(changed))}",
        )
    stray = unexpected_untracked(repo, output_rel, whitelist)
    if stray:
        _reject_late(
            receipt_path,
            f"unexpected untracked files outside the output directory: {', '.join(stray)}",
        )

    merged_env = dict(os.environ)
    if extra_env:
        merged_env.update(extra_env)
    if not merged_env.get("CARGO_TARGET_DIR"):
        _reject_late(
            receipt_path,
            "CARGO_TARGET_DIR must be set so no per-session target directory is created",
        )

    tree = commit_tree(repo, expected_commit)

    output_dir.mkdir(parents=True, exist_ok=True)

    projection = environment_projection(extra_env or {})
    versions = tool_versions(merged_env)

    inventory = derive_source_inventory(repo, expected_commit)
    inventory_path = output_dir / "source-inventory.json"
    write_json(inventory_path, inventory)

    recorded_checks: list[dict] = []
    family_seen: dict[str, int] = {}

    for index, check in enumerate(checks, start=1):
        occurrence = family_seen.get(check.family, 0) + 1
        family_seen[check.family] = occurrence
        with tempfile.TemporaryDirectory(prefix="phase18-baseline-") as tmp:
            temp_root = Path(tmp)
            child_env = dict(merged_env)
            child_env["TMPDIR"] = str(temp_root)
            before = _manifest_tree(temp_root)
            record = _run_check(check, f"{index:02d}", repo, child_env, output_dir, timeout)
            after = _manifest_tree(temp_root)
            record["temp_root_manifest_before"] = before
            record["temp_root_manifest_after"] = after
            record["id"] = f"{index:02d}"
            if record["census"]["residual_processes"]:
                recorded_checks.append(record)
                _write_evidence(output_dir, recorded_checks, receipt_path)
                _reject_late(
                    receipt_path,
                    f"residual process after check {index:02d} ({check.family})",
                )
        fixture_blob = git_blob(repo, expected_commit, check.fixture)
        record["fixture"] = {
            "path": check.fixture,
            "sha256": (
                sha256_bytes(normalize_newlines(fixture_blob))
                if fixture_blob is not None
                else None
            ),
        }
        record["env_projection"] = projection
        record["classification"] = (
            "pass" if record["exit"] == 0 and not record["timed_out"] else "fail"
        )
        recorded_checks.append(record)
        if record["classification"] != "pass":
            _write_evidence(output_dir, recorded_checks, receipt_path)
            _reject_late(
                receipt_path,
                f"failed command {index:02d} ({check.family}): exit {record['exit']}",
            )

    audit_code, audit_stdout, audit_stderr = audit_runner(repo, output_dir)
    audit_record = {
        "argv": [
            sys.executable,
            str(Path(__file__).resolve().parent / "opi-artifact-audit.py"),
            str(output_dir),
            "--workspace-root",
            str(repo),
            "--json",
        ],
        "exit": audit_code,
    }

    receipt = {
        "schema": RECEIPT_SCHEMA,
        "status": "pending-audit",
        "commit": expected_commit,
        "tree": tree,
        "captured_at": utc_now(),
        "helper": {"path": str(helper_path), "sha256": sha256_file(helper_path)},
        "test": {"path": str(test_path), "sha256": sha256_file(test_path)},
        "platform": {"system": sys.platform, "python": sys.version.split()[0]},
        "toolchain": versions,
        "environment": projection,
        "timeout_seconds": timeout,
        "checks": recorded_checks,
        "source_inventory": {
            "file": "source-inventory.json",
            "sha256": sha256_file(inventory_path),
            "derived_digest": source_inventory_digest(inventory),
        },
        "artifact_audit": audit_record,
    }

    if audit_code != 0:
        receipt["status"] = "rejected"
        receipt["reject_reason"] = "artifact audit failed"
        write_json(receipt_path, receipt)
        write_index(output_dir)
        raise CaptureRejected(
            f"artifact audit failed with exit {audit_code}: {audit_stderr.strip()[:500]}"
        )

    receipt["status"] = "ok"
    write_json(receipt_path, receipt)
    write_index(output_dir)
    return receipt


def _write_evidence(
    output_dir: Path, checks: list[dict], receipt_path: Path
) -> None:
    write_json(
        receipt_path,
        {
            "schema": RECEIPT_SCHEMA,
            "status": "rejected",
            "checks_started": len(checks),
        },
    )
    write_index(output_dir)


def write_index(output_dir: Path) -> None:
    files: dict[str, dict] = {}
    for path in sorted(output_dir.rglob("*")):
        if not path.is_file() or path.name == "index.json":
            continue
        rel = str(path.relative_to(output_dir)).replace("\\", "/")
        files[rel] = {"bytes": path.stat().st_size, "sha256": sha256_file(path)}
    write_json(output_dir / "index.json", {"schema": "phase18-baseline-index/1", "files": files})


# ---------------------------------------------------------------------------
# Verify
# ---------------------------------------------------------------------------


def verify(
    repo: Path,
    expected_commit: str,
    input_dir: Path,
    *,
    helper_path: Path | None = None,
    test_path: Path | None = None,
    audit_runner=None,
) -> list[str]:
    """Re-check persisted evidence without rerunning the historical runtime."""
    if helper_path is None:
        helper_path = Path(__file__).resolve()
    if test_path is None:
        test_path = (
            Path(__file__).resolve().parent
            / "test_capture_phase18_minimal_runtime_baseline.py"
        )
    if audit_runner is None:
        audit_runner = _default_audit_runner()

    problems: list[str] = []

    receipt_path = input_dir / "receipt.json"
    if not receipt_path.is_file():
        return [f"missing receipt: {receipt_path}"]
    receipt = load_json(receipt_path)
    if receipt.get("schema") != RECEIPT_SCHEMA:
        problems.append(f"unexpected receipt schema: {receipt.get('schema')}")
    if receipt.get("status") != "ok":
        problems.append(f"receipt status is {receipt.get('status')!r}, expected 'ok'")
        return problems

    if receipt.get("commit") != expected_commit:
        problems.append(
            f"commit mismatch: receipt {receipt.get('commit')} != expected {expected_commit}"
        )
    else:
        try:
            tree_now = commit_tree(repo, expected_commit)
        except CaptureRejected as error:
            problems.append(str(error))
            tree_now = None
        if tree_now is not None and receipt.get("tree") != tree_now:
            problems.append(
                f"tree mismatch: receipt {receipt.get('tree')} != {tree_now}"
            )

    if sha256_file(helper_path) != receipt.get("helper", {}).get("sha256"):
        problems.append(
            "helper digest mismatch: capture helper changed after evidence was bound"
        )
    if sha256_file(test_path) != receipt.get("test", {}).get("sha256"):
        problems.append(
            "helper test digest mismatch: capture test changed after evidence was bound"
        )

    index_path = input_dir / "index.json"
    if not index_path.is_file():
        problems.append("missing index.json")
        index = {"files": {}}
    else:
        index = load_json(index_path)
    listed: dict[str, dict] = index.get("files", {})

    on_disk = {
        str(path.relative_to(input_dir)).replace("\\", "/")
        for path in input_dir.rglob("*")
        if path.is_file() and path.name != "index.json"
    }
    for rel in sorted(on_disk - set(listed)):
        problems.append(f"raw evidence file not referenced by index: {rel}")
    for rel, meta in sorted(listed.items()):
        path = input_dir / rel
        if not path.is_file():
            problems.append(f"index references missing file: {rel}")
            continue
        if sha256_file(path) != meta.get("sha256"):
            problems.append(f"digest mismatch (index): {rel}")

    for check in receipt.get("checks", []):
        for field in ("stdout_file", "stderr_file"):
            rel = check.get(field)
            if not rel:
                problems.append(f"check {check.get('id')} missing {field}")
                continue
            path = input_dir / rel
            if not path.is_file():
                problems.append(f"check {check.get('id')} missing raw file {rel}")
            elif sha256_file(path) != check.get(f"{field.split('_')[0]}_sha256"):
                problems.append(f"digest mismatch ({field}): {rel}")
        if check.get("classification") != "pass":
            problems.append(f"check {check.get('id')} classification is not pass")
        if check.get("census", {}).get("residual_processes"):
            problems.append(f"check {check.get('id')} recorded residual processes")

    families = {check.get("family") for check in receipt.get("checks", [])}
    for family in sorted(set(REQUIRED_FAMILIES) - families):
        problems.append(f"missing behavior family: {family}")

    fixture_problems: list[str] = []
    for check in receipt.get("checks", []):
        fixture = check.get("fixture", {})
        blob = git_blob(repo, expected_commit, fixture.get("path", ""))
        digest = (
            sha256_bytes(normalize_newlines(blob)) if blob is not None else None
        )
        if digest != fixture.get("sha256"):
            fixture_problems.append(
                f"fixture digest mismatch: {fixture.get('path')}"
            )
    problems.extend(fixture_problems)

    inventory_path = input_dir / "source-inventory.json"
    if not inventory_path.is_file():
        problems.append("missing source-inventory.json")
    else:
        if sha256_file(inventory_path) != receipt.get("source_inventory", {}).get(
            "sha256"
        ):
            problems.append("digest mismatch (receipt): source-inventory.json")
        try:
            derived = derive_source_inventory(repo, expected_commit)
        except CaptureRejected as error:
            problems.append(f"source inventory could not be re-derived: {error}")
            derived = None
        if (
            derived is not None
            and source_inventory_digest(derived)
            != receipt.get("source_inventory", {}).get("derived_digest")
        ):
            problems.append(
                "source inventory drift: bound commit tree no longer derives the recorded inventory"
            )

    audit_code, _, _ = audit_runner(repo, input_dir)
    if audit_code != 0:
        problems.append(f"artifact audit failed with exit {audit_code}")

    return problems


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--expected-commit", required=True)
    parser.add_argument("--output")
    parser.add_argument("--input")
    parser.add_argument(
        "--require-clean-product-paths",
        action="store_true",
        help="accepted for capture mode; the guard is always enforced",
    )
    parser.add_argument("--verify", action="store_true")
    parser.add_argument(
        "--timeout", type=int, default=DEFAULT_TIMEOUT_SECONDS
    )
    args = parser.parse_args(argv)

    repo = Path.cwd()
    if args.verify:
        if not args.input:
            parser.error("--verify requires --input")
        problems = verify(repo, args.expected_commit, Path(args.input))
        if problems:
            for problem in problems:
                print(f"- {problem}", file=sys.stderr)
            print("verify: FAIL", file=sys.stderr)
            return 1
        print("verify: PASS")
        return 0

    if not args.output:
        parser.error("capture mode requires --output")
    if not args.require_clean_product_paths:
        parser.error("capture mode requires --require-clean-product-paths")
    try:
        receipt = capture(
            repo,
            args.expected_commit,
            Path(args.output),
            timeout=args.timeout,
        )
    except CaptureRejected as error:
        print(f"capture rejected: {error}", file=sys.stderr)
        return 1
    print(
        f"capture: PASS ({len(receipt['checks'])} checks bound to {receipt['commit']})"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
