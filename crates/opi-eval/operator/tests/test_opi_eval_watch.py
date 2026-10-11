#!/usr/bin/env python3
"""Offline contracts for the read-only evaluation watch."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))

spec = importlib.util.spec_from_file_location("operator_watch", SCRIPTS / "operator_watch.py")
watch_module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(watch_module)


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


class WatchStateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.evaluation = self.workspace / "evaluations/2026/10/baseline-fixture"
        self.leases = self.workspace / "leases"
        write(self.evaluation / "record.json", {"id": "baseline-fixture", "families": {}})

    def family(self, name="tb21"):
        return self.evaluation / "families" / name

    def test_completed_result_wins_immediately(self):
        (self.evaluation / "families/tb21/runs/attempt-0/trials").mkdir(parents=True)
        write(self.evaluation / "result-handoff.json", {"status": "complete"})
        code, reason, status = watch_module.evaluate_state(self.evaluation, self.leases, 0)
        self.assertEqual((code, reason), (0, "completed"))
        self.assertEqual(status["families"], {"tb21": 0})

    def test_not_running_without_lease_or_result(self):
        code, reason, _ = watch_module.evaluate_state(self.evaluation, self.leases, None)
        self.assertEqual((code, reason), (1, "not-running"))

    def test_crashed_operator_detected_by_dead_lease_pid(self):
        write(self.leases / "baseline-fixture.json", {"pid": 2**22 - 1})
        code, reason, _ = watch_module.evaluate_state(self.evaluation, self.leases, 1)
        self.assertEqual((code, reason), (2, "operator-crashed"))

    def test_live_operator_is_running_until_budget_expires(self):
        write(self.leases / "baseline-fixture.json", {"pid": os.getpid()})
        code, reason, _ = watch_module.evaluate_state(self.evaluation, self.leases, 2)
        self.assertEqual((code, reason), (124, "running"))
        write(self.family() / "batch.json", {"started_at": "2020-01-01T00:00:00+00:00", "seconds": 60})
        code, reason, status = watch_module.evaluate_state(self.evaluation, self.leases, 2)
        self.assertEqual((code, reason), (4, "budget-expired"))
        self.assertEqual(status["budget_family"], "tb21")

    def test_settled_receipts_are_counted_per_attempt(self):
        write(self.leases / "baseline-fixture.json", {"pid": os.getpid()})
        for attempt in ("attempt-0", "attempt-1"):
            for trial in ("a", "b"):
                (self.family() / "runs" / attempt / "trials" / trial).mkdir(parents=True)
                (self.family() / "runs" / attempt / "trials" / trial / "receipt.json").write_text("{}")
        _, _, status = watch_module.evaluate_state(self.evaluation, self.leases, 0)
        self.assertEqual(status["settled"], 4)

    def test_missing_record_is_a_watch_error(self):
        code, reason, _ = watch_module.evaluate_state(self.root / "nowhere", self.leases, None)
        self.assertEqual((code, reason), (5, "watch-error"))


class WatchLoopTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.evaluation = self.workspace / "evaluations/2026/10/baseline-fixture"
        write(self.evaluation / "record.json", {"id": "baseline-fixture", "families": {}})

    def test_stall_wakes_when_nothing_settles_and_no_container_runs(self):
        write(self.workspace / "leases/baseline-fixture.json", {"pid": os.getpid()})
        status = watch_module.watch(self.evaluation, self.workspace, maximum=30, poll=0.01, stall=0.05,
                                    container_counter=lambda: 0)
        self.assertEqual(status["exit_code"], 3)
        self.assertEqual(status["reason"], "stall-suspected")

    def test_container_activity_prevents_stall_and_max_wakes_healthy(self):
        write(self.workspace / "leases/baseline-fixture.json", {"pid": os.getpid()})
        status = watch_module.watch(self.evaluation, self.workspace, maximum=0.08, poll=0.01, stall=0.03,
                                    container_counter=lambda: 1)
        self.assertEqual(status["exit_code"], 124)
        self.assertEqual(status["reason"], "max-reached")

    def test_unobservable_engine_never_reports_stall(self):
        write(self.workspace / "leases/baseline-fixture.json", {"pid": os.getpid()})
        status = watch_module.watch(self.evaluation, self.workspace, maximum=0.08, poll=0.01, stall=0.03,
                                    container_counter=lambda: None)
        self.assertEqual(status["reason"], "max-reached")

    def test_completed_mid_watch_wakes_immediately(self):
        (self.evaluation / "result-handoff.json").write_text("{}", encoding="utf-8")
        status = watch_module.watch(self.evaluation, self.workspace, maximum=5, poll=0.01, stall=1,
                                    container_counter=lambda: 0)
        self.assertEqual((status["exit_code"], status["reason"]), (0, "completed"))

    def test_invalid_bounds_reject(self):
        from operator_process import OperatorError
        for kwargs in ({"maximum": 0, "poll": 0.05, "stall": 0.1},
                       {"maximum": 1, "poll": 0, "stall": 0.1},
                       {"maximum": 1, "poll": 0.05, "stall": 0.0}):
            with self.assertRaises(OperatorError):
                watch_module.watch(self.evaluation, self.workspace, **kwargs)


if __name__ == "__main__":
    unittest.main()
