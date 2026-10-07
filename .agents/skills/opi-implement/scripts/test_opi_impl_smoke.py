#!/usr/bin/env python3
"""Contract tests for the smoke wrappers' authoritative gate order.

Phase 18 task 18.16 fixes ONE authoritative Phase-exit order, owned by the
smoke wrappers' ``full`` mode on every host:

    python scripts/opi-doc-check.py
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

The order is a documented source contract (the registered Phase 18 design's
"Phase exit additionally requires the repository gates below in order"), so
the gate-order tests parse both wrappers statically: exact sequence, cross-platform
parity, and exactly-once occurrence. ``boot`` and ``scoped`` modes must
survive the change, and no wrapper may reintroduce a standalone workspace
build.

Isolated subprocess checks use mocked tools and an invalid mode to verify the
repository cache path, lease release, and caller-supplied target handling.

Usage:
    python .agents/skills/opi-implement/scripts/test_opi_impl_smoke.py
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
SH = SCRIPTS / "opi-impl-smoke.sh"
PS1 = SCRIPTS / "opi-impl-smoke.ps1"
REPOSITORY = SCRIPTS.parents[3]
CACHE_TOOL = REPOSITORY / "scripts" / "opi-cargo-cache.py"

AUTHORITATIVE_ORDER = [
    "doc_check",
    "fmt",
    "clippy_all_targets",
    "test_all_targets",
    "test_doc",
    "rustdoc",
]

# One canonical gate per literal invocation text.
GATE_PATTERNS: list[tuple[str, re.Pattern[str]]] = [
    ("doc_check", re.compile(r"^python\s+scripts/opi-doc-check\.py$")),
    ("fmt", re.compile(r"^cargo fmt --check --all$")),
    ("clippy_all_targets", re.compile(
        r"^cargo clippy --workspace --all-targets -- -D warnings$")),
    ("test_all_targets", re.compile(r"^cargo test --workspace --all-targets$")),
    ("test_doc", re.compile(r"^cargo test --workspace --doc$")),
    ("rustdoc", re.compile(r"^cargo doc --workspace --no-deps$")),
]

# bash sets RUSTDOCFLAGS inline on the rustdoc invocation.
SH_RUSTDOC_INLINE = 'RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps'
# PowerShell exports RUSTDOCFLAGS on its own line directly above the command.
PS1_RUSTDOC_ENV = '$env:RUSTDOCFLAGS = "-D warnings"'


def read_source(path: Path) -> str:
    """Wrapper text with CRLF folded to LF so line-based contracts hold."""
    return path.read_text(encoding="utf-8").replace("\r\n", "\n")


def full_branch_sh() -> str:
    source = read_source(SH)
    match = re.search(r"^  full\)\n(.*?)^  \w", source, re.MULTILINE | re.DOTALL)
    assert match, "full) branch not found in opi-impl-smoke.sh"
    return match.group(1)


def full_branch_ps1() -> str:
    source = read_source(PS1)
    match = re.search(r'^  "full" \{\n(.*?)\n  \}', source, re.DOTALL | re.MULTILINE)
    assert match, '"full" branch not found in opi-impl-smoke.ps1'
    return match.group(1)


def gate_sequence(branch: str) -> list[str]:
    """Canonical gate ids in first-occurrence line order.

    POSIX wrapper lines carry `2>&1 || { ...; }` suffixes; the gate contract
    is about the invocation itself, so the suffix is stripped first.
    """
    sequence: list[str] = []
    for raw in branch.splitlines():
        line = raw.strip().split(" 2>&1 ", 1)[0].strip()
        # bash inline env prefix folds into the rustdoc gate line.
        if line == SH_RUSTDOC_INLINE:
            sequence.append("rustdoc")
            continue
        for gate, pattern in GATE_PATTERNS:
            if pattern.match(line):
                sequence.append(gate)
                break
    return sequence


class SmokeWrapperOrder(unittest.TestCase):
    def test_sh_full_runs_the_authoritative_order(self) -> None:
        self.assertEqual(gate_sequence(full_branch_sh()), AUTHORITATIVE_ORDER)

    def test_ps1_full_runs_the_authoritative_order(self) -> None:
        self.assertEqual(gate_sequence(full_branch_ps1()), AUTHORITATIVE_ORDER)

    def test_full_gates_run_exactly_once_per_platform(self) -> None:
        for name, branch in (
            ("sh", full_branch_sh()),
            ("ps1", full_branch_ps1()),
        ):
            sequence = gate_sequence(branch)
            for gate in AUTHORITATIVE_ORDER:
                self.assertEqual(
                    sequence.count(gate), 1,
                    f"{name}: gate {gate!r} must occur exactly once in full",
                )

    def test_rustdoc_env_is_bound_to_the_rustdoc_gate(self) -> None:
        # bash: inline prefix on the same line.
        self.assertIn(SH_RUSTDOC_INLINE, full_branch_sh())
        # PowerShell: exported on the line directly above the invocation.
        ps1_lines = [line.strip() for line in full_branch_ps1().splitlines()]
        self.assertIn(PS1_RUSTDOC_ENV, ps1_lines)
        index = ps1_lines.index(PS1_RUSTDOC_ENV)
        self.assertEqual(ps1_lines[index + 1], "cargo doc --workspace --no-deps")

    def test_boot_and_scoped_modes_survive(self) -> None:
        sh = read_source(SH)
        ps1 = read_source(PS1)
        self.assertIn("boot)", sh)
        self.assertIn("scoped)", sh)
        self.assertIn('"boot"', ps1)
        self.assertIn('"scoped"', ps1)
        self.assertIn("--lib --bins", sh)
        self.assertIn("--lib --bins", ps1)

    def test_no_standalone_workspace_build(self) -> None:
        for source in (read_source(SH), read_source(PS1)):
            self.assertNotIn("cargo build --workspace", source)


class SmokeWrapperCachePaths(unittest.TestCase):
    def assert_cache_calls(self, calls: list[list[str]], target: str) -> None:
        """An invalid mode acquires and releases only the canonical cache lease."""
        self.assertEqual(calls[0], [str(CACHE_TOOL), "resolve"])
        self.assertEqual(len(calls), 3)
        self.assertEqual(calls[1][:6], [
            str(CACHE_TOOL), "lease", "start", "--target", target, "--pid",
        ])
        self.assertEqual(calls[2][:6], [
            str(CACHE_TOOL), "lease", "end", "--target", target, "--pid",
        ])
        self.assertEqual(len(calls[1]), 7)
        self.assertEqual(calls[1][-1], calls[2][-1])

    def test_ps1_cache_path_and_caller_target(self) -> None:
        powershell = shutil.which("powershell") or shutil.which("pwsh")
        if not powershell:
            self.skipTest("PowerShell is not installed on this host")
        with tempfile.TemporaryDirectory(prefix="opi-smoke-") as temp:
            scratch = Path(temp)
            log = scratch / "calls.jsonl"
            target = str(scratch / "cargo-target")
            launcher = scratch / "mock-smoke.ps1"
            launcher.write_text(
                "function rustc { $global:LASTEXITCODE = 0 }\n"
                "function cargo {\n"
                "  if ($args.Count -ne 1 -or $args[0] -ne '--version') {\n"
                "    throw 'unexpected Cargo gate'\n"
                "  }\n"
                "  $global:LASTEXITCODE = 0\n"
                "}\n"
                "function python {\n"
                "  ConvertTo-Json -InputObject @($args) -Compress |\n"
                "    Add-Content -LiteralPath $env:SMOKE_LOG -Encoding UTF8\n"
                "  if ($args[1] -eq 'resolve') { $env:SMOKE_TARGET }\n"
                "  $global:LASTEXITCODE = 0\n"
                "}\n"
                "& $env:SMOKE_WRAPPER invalid-mode\n",
                encoding="utf-8",
            )
            for preset in (False, True):
                with self.subTest(caller_target=preset):
                    log.write_text("", encoding="utf-8")
                    env = os.environ.copy()
                    env.pop("CARGO_TARGET_DIR", None)
                    env.update(SMOKE_WRAPPER=str(PS1), SMOKE_LOG=str(log),
                               SMOKE_TARGET=target)
                    if preset:
                        env["CARGO_TARGET_DIR"] = target
                    result = subprocess.run(
                        [powershell, "-NoProfile", "-NonInteractive",
                         "-ExecutionPolicy", "Bypass", "-File", str(launcher)],
                        cwd=scratch, env=env, capture_output=True, text=True,
                        timeout=30,
                    )
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("FAIL: unknown mode 'invalid-mode'",
                                  result.stdout + result.stderr)
                    calls = [json.loads(line) for line in
                             log.read_text(encoding="utf-8-sig").splitlines()]
                    if preset:
                        self.assertEqual(calls, [])
                    else:
                        self.assert_cache_calls(calls, target)

    @unittest.skipIf(os.name == "nt", "run bash checks on a POSIX host")
    def test_sh_cache_path_and_caller_target(self) -> None:
        bash = shutil.which("bash")
        if not bash:
            self.skipTest("bash is not installed on this host")
        with tempfile.TemporaryDirectory(prefix="opi-smoke-") as temp:
            scratch = Path(temp)
            log = scratch / "calls.tsv"
            target = str(scratch / "cargo-target")
            launcher = scratch / "mock-smoke.sh"
            launcher.write_text(
                "rustc() { return 0; }\n"
                "cargo() {\n"
                "  [ \"$#\" -eq 1 ] && [ \"$1\" = --version ] || return 99\n"
                "}\n"
                "python() {\n"
                "  printf '%s' \"$1\" >> \"$SMOKE_LOG\"\n"
                "  shift\n"
                "  printf '\\t%s' \"$@\" >> \"$SMOKE_LOG\"\n"
                "  printf '\\n' >> \"$SMOKE_LOG\"\n"
                "  if [ \"$1\" = resolve ]; then printf '%s\\n' \"$SMOKE_TARGET\"; fi\n"
                "}\n"
                "export -f rustc cargo python\n"
                "exec bash \"$SMOKE_WRAPPER\" invalid-mode\n",
                encoding="utf-8",
            )
            for preset in (False, True):
                with self.subTest(caller_target=preset):
                    log.write_text("", encoding="utf-8")
                    env = os.environ.copy()
                    env.pop("CARGO_TARGET_DIR", None)
                    env.update(SMOKE_WRAPPER=str(SH), SMOKE_LOG=str(log),
                               SMOKE_TARGET=target)
                    if preset:
                        env["CARGO_TARGET_DIR"] = target
                    result = subprocess.run(
                        [bash, str(launcher)], cwd=scratch, env=env,
                        capture_output=True, text=True, timeout=30,
                    )
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("FAIL: unknown mode 'invalid-mode'",
                                  result.stdout + result.stderr)
                    calls = [line.split("\t") for line in
                             log.read_text(encoding="utf-8").splitlines()]
                    if preset:
                        self.assertEqual(calls, [])
                    else:
                        self.assert_cache_calls(calls, target)


if __name__ == "__main__":
    unittest.main(verbosity=2)
