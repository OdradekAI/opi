from __future__ import annotations

import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().with_name("check_test_execution.py")
SPEC = importlib.util.spec_from_file_location("audit_test_execution", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def inventory(*names: str) -> str:
    return "\n".join([*(f"{name}: test" for name in names), "",
                      f"{len(names)} tests, 0 benchmarks", ""])


def run_log(*rows: tuple[str, str], filtered: int = 0) -> str:
    counts = {"ok": 0, "FAILED": 0, "ignored": 0}
    for _, status in rows:
        counts[status] += 1
    state = "FAILED" if counts["FAILED"] else "ok"
    return "\n".join([
        f"running {len(rows)} tests",
        *(f"test {name} ... {status}" for name, status in rows), "",
        f"test result: {state}. {counts['ok']} passed; {counts['FAILED']} failed; "
        f"{counts['ignored']} ignored; 0 measured; {filtered} filtered out; "
        "finished in 0.01s", "",
    ])


class ExecutionCheckTests(unittest.TestCase):
    def check(self, running: str, **kwargs: object) -> dict:
        return checker.check_execution(
            inventory("fixture", "native", "other"), inventory("native"),
            running, **kwargs,
        )

    def test_exact_normal_filter_has_one_executed_case(self) -> None:
        result = self.check(run_log(("fixture", "ok"), filtered=2),
                            test_filter="fixture", exact=True)
        self.assertEqual("PASS", result["status"])
        self.assertEqual(1, result["eligible"])
        self.assertEqual(1, result["observed"]["executed"])

    def test_ignored_selection_runs_the_ignored_case(self) -> None:
        result = self.check(run_log(("native", "ok"), filtered=2),
                            test_filter="native", exact=True, selection="ignored")
        self.assertEqual("PASS", result["status"])

    def test_normal_run_accounts_for_ignored_cases_without_counting_execution(self) -> None:
        log = run_log(("fixture", "ok"), ("native", "ignored"), ("other", "ok"))
        log = log.replace("... ignored\n", "... ignored, requires local material\n")
        result = self.check(log)
        self.assertEqual("PASS", result["status"])
        self.assertEqual(2, result["observed"]["executed"])
        self.assertEqual(1, result["observed"]["ignored"])

    def test_include_ignored_requires_every_matching_case(self) -> None:
        result = self.check(run_log(("fixture", "ok"), ("native", "ok"),
                                   ("other", "ok")), selection="include-ignored")
        self.assertEqual("PASS", result["status"])
        self.assertEqual(3, result["eligible"])

    def test_substring_filter_and_exact_filter_select_different_sets(self) -> None:
        all_tests = inventory("case", "case_more")
        result = checker.check_execution(all_tests, inventory(),
                                         run_log(("case", "ok"), ("case_more", "ok")),
                                         test_filter="case")
        self.assertEqual("PASS", result["status"])
        exact = checker.check_execution(all_tests, inventory(),
                                        run_log(("case", "ok"), filtered=1),
                                        test_filter="case", exact=True)
        self.assertEqual("PASS", exact["status"])
        self.assertEqual(1, exact["matching"])

    def test_missing_filter_fails_despite_zero_exit_summary(self) -> None:
        result = self.check(run_log(filtered=3), test_filter="missing", exact=True)
        self.assertEqual("FAIL", result["status"])
        self.assertIn("filter matches no listed test", result["errors"])

    def test_stale_ignored_filter_cannot_claim_fixture_as_native_execution(self) -> None:
        result = self.check(run_log(filtered=3), test_filter="fixture", exact=True,
                            selection="ignored")
        self.assertEqual("FAIL", result["status"])
        self.assertEqual(1, result["matching"])
        self.assertEqual(0, result["eligible"])
        self.assertIn("selection has no runnable tests", result["errors"])

    def test_ignored_only_normal_selection_is_not_execution(self) -> None:
        result = self.check(run_log(("native", "ignored"), filtered=2),
                            test_filter="native", exact=True)
        self.assertEqual("FAIL", result["status"])
        self.assertEqual(0, result["observed"]["executed"])

    def test_failed_behavior_is_counted_but_does_not_pass(self) -> None:
        result = self.check(run_log(("fixture", "FAILED"), filtered=2),
                            test_filter="fixture", exact=True)
        self.assertEqual("FAIL", result["status"])
        self.assertEqual(1, result["observed"]["executed"])
        self.assertIn("executed tests failed", result["errors"])

    def test_wrong_case_cannot_replace_expected_case_with_same_count(self) -> None:
        result = self.check(run_log(("other", "ok"), filtered=2),
                            test_filter="fixture", exact=True)
        self.assertEqual("FAIL", result["status"])
        self.assertIn("executed identities differ from selected runnable tests", result["errors"])

    def test_filtered_count_and_expected_count_are_checked(self) -> None:
        result = self.check(run_log(("fixture", "ok"), filtered=1),
                            test_filter="fixture", exact=True, expected_count=2)
        self.assertEqual("FAIL", result["status"])
        self.assertIn("filtered count differs from inventory selection", result["errors"])
        self.assertIn("expected count differs from selected runnable tests", result["errors"])

    def test_report_count_cannot_replace_missing_case_observation(self) -> None:
        log = run_log(("fixture", "ok"), filtered=2).replace("test fixture ... ok\n", "")
        with self.assertRaisesRegex(checker.LogError, "case observations"):
            self.check(log)

    def test_duplicate_case_or_inventory_entries_are_rejected(self) -> None:
        with self.assertRaisesRegex(checker.LogError, "duplicate test"):
            self.check(run_log(("fixture", "ok"), ("fixture", "ok"), filtered=1))
        with self.assertRaisesRegex(checker.LogError, "duplicate test"):
            checker.check_execution(inventory("a", "a"), inventory(), run_log())

    def test_inventory_total_and_ignored_subset_cannot_drift(self) -> None:
        with self.assertRaisesRegex(checker.LogError, "inventory count"):
            checker.check_execution(inventory("a").replace("1 tests", "2 tests"),
                                    inventory(), run_log())
        with self.assertRaisesRegex(checker.LogError, "ignored inventory"):
            checker.check_execution(inventory("a"), inventory("unknown"), run_log())

    def test_multiple_targets_missing_summary_and_benchmarks_fail_closed(self) -> None:
        for listing, running in [
            (inventory("a") + inventory("b"), run_log()),
            (inventory("a"), "running 1 test\n"),
            (inventory("a").replace("0 benchmarks", "1 benchmark"), run_log()),
            (inventory("a"), run_log() + run_log()),
        ]:
            with self.subTest(listing=listing, running=running):
                with self.assertRaises(checker.LogError):
                    checker.check_execution(listing, inventory(), running)

    def test_empty_inventory_is_not_behavioral_acceptance(self) -> None:
        result = checker.check_execution(inventory(), inventory(), run_log())
        self.assertEqual("FAIL", result["status"])
        self.assertEqual(0, result["observed"]["executed"])


class ExecutionCliTests(unittest.TestCase):
    def invoke(self, root: Path, *options: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([
            sys.executable, str(SCRIPT), "--list-output", str(root / "all.txt"),
            "--ignored-list-output", str(root / "ignored.txt"),
            "--run-output", str(root / "run.txt"), *options,
        ], capture_output=True, text=True, check=False, timeout=10)

    def seed(self, root: Path, *, running: str) -> None:
        (root / "all.txt").write_text(inventory("fixture", "native"), encoding="utf-8")
        (root / "ignored.txt").write_text(inventory("native"), encoding="utf-8")
        (root / "run.txt").write_text(running, encoding="utf-8")

    def test_cli_stale_filter_exits_two_and_preserves_every_input(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.seed(root, running=run_log(filtered=2))
            before = {p.name: p.read_bytes() for p in root.iterdir()}
            result = self.invoke(root, "--filter", "fixture", "--exact",
                                 "--selection", "ignored")
            self.assertEqual(2, result.returncode, result.stderr)
            value = json.loads(result.stdout)
            self.assertEqual(0, value["eligible"])
            self.assertEqual(0, value["observed"]["executed"])
            self.assertEqual(before, {p.name: p.read_bytes() for p in root.iterdir()})

    def test_cli_positive_includes_exact_input_digests(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.seed(root, running=run_log(("fixture", "ok"), filtered=1))
            result = self.invoke(root, "--filter", "fixture", "--exact",
                                 "--expected-count", "1")
            self.assertEqual(0, result.returncode, result.stderr)
            value = json.loads(result.stdout)
            self.assertEqual("PASS", value["status"])
            for key in ("list_output", "ignored_list_output", "run_output"):
                raw = Path(value["inputs"][key]["path"]).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(),
                                 value["inputs"][key]["sha256"])

    def test_cli_truncated_invalid_encoding_and_missing_logs_are_failures(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.seed(root, running="private-marker-without-summary\n")
            truncated = self.invoke(root)
            self.assertEqual(2, truncated.returncode)
            self.assertNotIn("private-marker", truncated.stdout + truncated.stderr)
            (root / "run.txt").write_bytes(b"\xff")
            invalid = self.invoke(root)
            self.assertEqual(2, invalid.returncode)
            self.assertEqual("FAIL", json.loads(invalid.stdout)["status"])
            (root / "run.txt").unlink()
            missing = self.invoke(root)
            self.assertEqual(2, missing.returncode)

    def test_cli_rejects_oversized_nul_and_unbounded_numeric_logs(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.seed(root, running=run_log(("fixture", "ok"), filtered=1))
            run_path = root / "run.txt"
            with run_path.open("wb") as stream:
                stream.truncate(16 * 1024 * 1024 + 1)
            oversized = self.invoke(root)
            self.assertEqual(2, oversized.returncode)
            self.assertEqual("FAIL", json.loads(oversized.stdout)["status"])
            run_path.write_bytes(b"private-marker\x00\n")
            nul = self.invoke(root)
            self.assertEqual(2, nul.returncode)
            self.assertNotIn("private-marker", nul.stdout + nul.stderr)
            run_path.write_text(run_log(("fixture", "ok"), filtered=1), encoding="utf-8")
            (root / "all.txt").write_text("9" * 5000 + " tests, 0 benchmarks\n",
                                           encoding="utf-8")
            numeric = self.invoke(root)
            self.assertEqual(2, numeric.returncode, numeric.stderr)
            self.assertEqual("FAIL", json.loads(numeric.stdout)["status"])
            self.seed(root, running=run_log(("fixture", "ok"), filtered=1)
                      .replace("1 passed", "9" * 5000 + " passed"))
            numeric_run = self.invoke(root)
            self.assertEqual(2, numeric_run.returncode, numeric_run.stderr)
            self.assertEqual("FAIL", json.loads(numeric_run.stdout)["status"])

    def test_cli_rejects_nonpositive_expected_count_and_exact_without_filter(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.seed(root, running=run_log(("fixture", "ok"), filtered=1))
            for options in [("--expected-count", "0"), ("--exact",)]:
                with self.subTest(options=options):
                    result = self.invoke(root, *options)
                    self.assertEqual(2, result.returncode)


if __name__ == "__main__":
    unittest.main()
