#!/usr/bin/env python3
"""Contract tests for the Phase 18 Minimal Runtime baseline capture helper.

The helper under test binds a pre-phase18 behavior snapshot of the Reference
Product to an exact start commit and tree. These tests exercise the guard,
census, receipt, and verify contracts with hermetic temporary Git repositories
and stub commands; they never invoke cargo or the real product build.
"""

from __future__ import annotations

import importlib.util
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("capture-phase18-minimal-runtime-baseline.py")
SPEC = importlib.util.spec_from_file_location(
    "capture_phase18_minimal_runtime_baseline", SCRIPT
)
assert SPEC is not None and SPEC.loader is not None
helper = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(helper)

HELPER_TEST_PATH = Path(__file__).name

FAMILIES = (
    "ordinary-cli-io",
    "minimal-runtime-evidence",
    "user-policy",
    "provider-routing",
    "tool-behavior",
    "session-persistence",
    "background-process-cleanup",
)


def git(repo: Path, *args: str) -> str:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    ).stdout


def init_repo(repo: Path) -> str:
    """Create a minimal product-shaped repo and return its first commit."""
    (repo / "Cargo.toml").write_text(
        "[workspace]\n"
        'members = ["crates/opi-coding-agent"]\n'
        'resolver = "2"\n',
        encoding="utf-8",
        newline="\n",
    )
    crate = repo / "crates" / "opi-coding-agent"
    crate.mkdir(parents=True)
    (crate / "Cargo.toml").write_text(
        "[package]\n"
        'name = "opi-coding-agent"\n'
        'version = "0.8.1"\n',
        encoding="utf-8",
        newline="\n",
    )
    src = crate / "src"
    src.mkdir()
    (src / "main.rs").write_text("fn main() {}\n", encoding="utf-8", newline="\n")
    (repo / "tests").mkdir()
    (repo / "tests" / "binary.rs").write_text(
        "// fixture\n", encoding="utf-8", newline="\n"
    )
    git(repo, "init", "--quiet")
    git(
        repo,
        "-c",
        "user.email=t@example.invalid",
        "-c",
        "user.name=t",
        "add",
        "Cargo.toml",
        "crates",
        "tests",
    )
    git(
        repo,
        "-c",
        "user.email=t@example.invalid",
        "-c",
        "user.name=t",
        "commit",
        "--quiet",
        "-m",
        "base",
    )
    return git(repo, "rev-parse", "HEAD").strip()


def stub_checks(ok: bool = True) -> list[helper.Check]:
    """One stub command per required behavior family."""
    code = "print('ok')" if ok else "import sys; sys.exit(3)"
    argv = [sys.executable, "-c", code]
    fixture = "tests/binary.rs"
    return [helper.Check(family, argv, fixture) for family in FAMILIES]


def stub_audit(ok: bool = True):
    def run(repo: Path, artifact_dir: Path) -> tuple[int, str, str]:
        return (0 if ok else 1, "{}", "")

    return run


class GuardTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name)
        self.commit = init_repo(self.repo)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_clean_tree_passes(self) -> None:
        changed = helper.changed_product_paths(
            self.repo, self.commit, frozenset({SCRIPT.name, HELPER_TEST_PATH})
        )
        self.assertEqual(changed, set())

    def test_pure_line_ending_churn_is_not_product_drift(self) -> None:
        target = self.repo / "crates" / "opi-coding-agent" / "src" / "main.rs"
        target.write_bytes(target.read_bytes().replace(b"\n", b"\r\n"))
        changed = helper.changed_product_paths(
            self.repo, self.commit, frozenset({SCRIPT.name, HELPER_TEST_PATH})
        )
        self.assertEqual(changed, set())

    def test_semantic_edit_is_rejected(self) -> None:
        target = self.repo / "crates" / "opi-coding-agent" / "src" / "main.rs"
        target.write_text(
            "fn main() { /* changed */ }\n", encoding="utf-8", newline="\n"
        )
        changed = helper.changed_product_paths(
            self.repo, self.commit, frozenset({SCRIPT.name, HELPER_TEST_PATH})
        )
        self.assertIn(
            "crates/opi-coding-agent/src/main.rs", changed,
        )

    def test_whitelist_covers_helper_test_and_ledger(self) -> None:
        (self.repo / SCRIPT.name).write_text("# helper\n", encoding="utf-8")
        (self.repo / HELPER_TEST_PATH).write_text("# test\n", encoding="utf-8")
        (self.repo / ".opi-impl-state.json").write_text("{}", encoding="utf-8")
        changed = helper.changed_product_paths(
            self.repo,
            self.commit,
            frozenset({SCRIPT.name, HELPER_TEST_PATH, ".opi-impl-state.json"}),
        )
        self.assertEqual(changed, set())

    def test_untracked_file_outside_output_is_rejected(self) -> None:
        output = "evidence/out"
        (self.repo / "stray.txt").write_text("x", encoding="utf-8")
        (self.repo / "evidence" / "out").mkdir(parents=True)
        (self.repo / "evidence" / "out" / "stdout.log").write_text(
            "x", encoding="utf-8"
        )
        stray = helper.unexpected_untracked(self.repo, output)
        self.assertEqual(stray, ["stray.txt"])


class ReceiptSourceDigestTests(unittest.TestCase):
    def test_relocated_verifier_uses_the_receipt_bound_historical_blob(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            init_repo(repo)
            historical_path = "scripts/capture-baseline.py"
            historical = repo / historical_path
            historical.parent.mkdir()
            historical.write_bytes(b"# receipt-bound verifier\n")
            git(repo, "add", historical_path)
            git(
                repo,
                "-c",
                "user.email=t@example.invalid",
                "-c",
                "user.name=t",
                "commit",
                "--quiet",
                "-m",
                "bind verifier",
            )
            verifier_commit = git(repo, "rev-parse", "HEAD").strip()
            active = repo / "crates" / "opi-eval" / "scripts" / historical.name
            active.parent.mkdir(parents=True)
            active.write_bytes(b"# relocated verifier\n")
            expected = helper.sha256_bytes(historical.read_bytes())

            actual = helper._receipt_bound_source_digest(
                repo,
                active,
                expected,
                historical_revision=verifier_commit,
                historical_path=historical_path,
            )

            self.assertEqual(actual, expected)


class CensusTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform.startswith("linux"), "requires /proc")
    def test_census_detects_and_clears_residual_child(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            proc = subprocess.Popen(
                ["sleep", "30"], start_new_session=True, cwd=str(root)
            )
            try:
                deadline = time.monotonic() + 5.0
                found = []
                while time.monotonic() < deadline:
                    found = helper.census(root)
                    if found:
                        break
                    time.sleep(0.05)
                self.assertEqual(len(found), 1)
                self.assertEqual(found[0]["pid"], proc.pid)
            finally:
                os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                proc.wait()
            self.assertEqual(helper.census(root), [])


class SourceInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name)
        self.commit = init_repo(self.repo)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_inventory_records_dependencies_and_zero_eval_references(self) -> None:
        inventory = helper.derive_source_inventory(self.repo, self.commit)
        tables = inventory["members"]["crates/opi-coding-agent"][
            "dependency_tables"
        ]
        self.assertEqual(
            tables,
            {"dependencies": [], "dev-dependencies": [], "build-dependencies": []},
        )
        self.assertEqual(inventory["companion_references"]["count"], 0)
        self.assertEqual(inventory["companion_references"]["paths"], [])
        self.assertTrue(inventory["anchor_digests"])

    def test_inventory_digest_changes_when_source_changes(self) -> None:
        first = helper.source_inventory_digest(
            helper.derive_source_inventory(self.repo, self.commit)
        )
        target = self.repo / "crates" / "opi-coding-agent" / "src" / "main.rs"
        target.write_text(
            "fn main() { /* changed */ }\n", encoding="utf-8", newline="\n"
        )
        git(
            self.repo,
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "user.name=t",
            "commit",
            "--quiet",
            "-am",
            "change",
        )
        second_commit = git(self.repo, "rev-parse", "HEAD").strip()
        second = helper.source_inventory_digest(
            helper.derive_source_inventory(self.repo, second_commit)
        )
        self.assertNotEqual(first, second)


@unittest.skipUnless(sys.platform.startswith("linux"), "census requires /proc")
class CaptureVerifyTests(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self._tmp.name)
        self.commit = init_repo(self.repo)
        self.out = self.repo / "evidence" / "out"
        self.helper_copy = self.repo / "fake-helper.py"
        self.helper_copy.write_text("# helper\n", encoding="utf-8")
        self.test_copy = self.repo / "fake-test.py"
        self.test_copy.write_text("# test\n", encoding="utf-8")
        self.extra_env = {
            "CARGO_TARGET_DIR": str(self.repo / "target"),
            "TMPDIR": tempfile.gettempdir(),
        }

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def _capture(self, checks=None, audit=None) -> dict:
        return helper.capture(
            self.repo,
            self.commit,
            self.out,
            checks=stub_checks() if checks is None else checks,
            helper_path=self.helper_copy,
            test_path=self.test_copy,
            audit_runner=stub_audit() if audit is None else audit,
            timeout=60,
            extra_env=self.extra_env,
            whitelist=frozenset({"fake-helper.py", "fake-test.py"}),
        )

    def _verify(self, expected_commit=None) -> list[str]:
        return helper.verify(
            self.repo,
            self.commit if expected_commit is None else expected_commit,
            self.out,
            helper_path=self.helper_copy,
            test_path=self.test_copy,
            audit_runner=stub_audit(),
        )

    def test_roundtrip_capture_then_verify(self) -> None:
        receipt = self._capture()
        self.assertEqual(receipt["status"], "ok")
        self.assertEqual(receipt["commit"], self.commit)
        self.assertEqual(len(receipt["checks"]), len(FAMILIES))
        for check in receipt["checks"]:
            self.assertEqual(check["classification"], "pass")
            self.assertEqual(check["census"]["residual_processes"], [])
        index = helper.load_json(self.out / "index.json")
        listed = set(index["files"])
        on_disk = {
            str(p.relative_to(self.out))
            for p in self.out.rglob("*")
            if p.is_file() and p.name != "index.json"
        }
        self.assertEqual(listed, on_disk)
        self.assertIn("receipt.json", listed)
        self.assertEqual(self._verify(), [])

    def test_verify_rejects_tampered_raw_output(self) -> None:
        self._capture()
        target = next(self.out.glob("checks/*/stdout.log"))
        target.write_text("tampered\n", encoding="utf-8")
        problems = self._verify()
        self.assertTrue(any("digest mismatch" in p for p in problems))

    def test_verify_rejects_missing_index_entry(self) -> None:
        self._capture()
        target = next(self.out.glob("checks/*/stdout.log"))
        index_path = self.out / "index.json"
        index = helper.load_json(index_path)
        index["files"] = {
            k: v for k, v in index["files"].items() if k not in {"a", "b"}
        }
        rel = str(target.relative_to(self.out)).replace("\\", "/")
        index["files"].pop(rel, None)
        helper.write_json(index_path, index)
        problems = self._verify()
        self.assertTrue(any("not referenced" in p for p in problems))

    def test_verify_rejects_commit_mismatch(self) -> None:
        self._capture()
        other = "0" * 40
        problems = helper.verify(
            self.repo,
            other,
            self.out,
            helper_path=self.helper_copy,
            test_path=self.test_copy,
            audit_runner=stub_audit(),
        )
        self.assertTrue(any("commit mismatch" in p for p in problems))

    def test_verify_rejects_changed_helper(self) -> None:
        self._capture()
        self.helper_copy.write_text("# helper edited\n", encoding="utf-8")
        problems = self._verify()
        self.assertTrue(any("helper digest mismatch" in p for p in problems))

    def test_capture_rejects_dirty_product_path(self) -> None:
        target = self.repo / "crates" / "opi-coding-agent" / "src" / "main.rs"
        target.write_text(
            "fn main() { /* changed */ }\n", encoding="utf-8", newline="\n"
        )
        with self.assertRaises(helper.CaptureRejected) as ctx:
            self._capture()
        self.assertIn("dirty product path", str(ctx.exception))

    def test_capture_rejects_failed_command(self) -> None:
        with self.assertRaises(helper.CaptureRejected) as ctx:
            self._capture(checks=stub_checks(ok=False))
        self.assertIn("failed command", str(ctx.exception))

    def test_capture_rejects_missing_behavior_family(self) -> None:
        with self.assertRaises(helper.CaptureRejected) as ctx:
            self._capture(checks=stub_checks()[1:])
        self.assertIn("missing behavior family", str(ctx.exception))

    def test_capture_rejects_audit_failure(self) -> None:
        with self.assertRaises(helper.CaptureRejected) as ctx:
            self._capture(audit=stub_audit(ok=False))
        self.assertIn("artifact audit", str(ctx.exception))

    def test_capture_requires_cargo_target_dir(self) -> None:
        env = dict(self.extra_env)
        env.pop("CARGO_TARGET_DIR")
        with self.assertRaises(helper.CaptureRejected) as ctx:
            helper.capture(
                self.repo,
                self.commit,
                self.out,
                checks=stub_checks(),
                helper_path=self.helper_copy,
                test_path=self.test_copy,
                audit_runner=stub_audit(),
                timeout=60,
                extra_env=env,
                whitelist=frozenset({"fake-helper.py", "fake-test.py"}),
            )
        self.assertIn("CARGO_TARGET_DIR", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
