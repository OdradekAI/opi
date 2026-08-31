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
these tests parse both wrappers statically: exact sequence, cross-platform
parity, and exactly-once occurrence. ``boot`` and ``scoped`` modes must
survive the change, and no wrapper may reintroduce a standalone workspace
build.

Usage:
    python scripts/test_opi_impl_smoke.py
"""

from __future__ import annotations

import re
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
SH = SCRIPTS / "opi-impl-smoke.sh"
PS1 = SCRIPTS / "opi-impl-smoke.ps1"

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


if __name__ == "__main__":
    unittest.main(verbosity=2)
