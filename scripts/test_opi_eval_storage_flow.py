#!/usr/bin/env python3
"""Offline storage/configuration/process integration; no providers or Docker."""
import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[1]
RUNTIME = REPO / "crates/opi-eval/operator"
sys.path.insert(0, str(RUNTIME))

from operator_config import resolve, dumps_toml
from operator_process import OperatorError
import test_opi_eval_operator


class StorageFlowTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def profile(self, version=2):
        case = test_opi_eval_operator.OperatorTests()
        case.root = self.root
        value = case.profile()
        value["benchmarks"]["tb21"]["seeds"] = {}
        if version == 2:
            value["schema"] = "opi-eval-operator/2"
            value["baseline"]["max_age_days"] = 0
            value["retention"] = {"keep_latest_per_group": 3}
        return value

    def test_profile_two_keeps_one_config_for_freshness_and_retention(self):
        try:
            result = resolve(self.profile())
        except OperatorError as exc:
            self.fail("versioned retention profile is not supported: " + str(exc))
        self.assertEqual(result["retention"], {"keep_latest_per_group": 3})
        self.assertEqual(result["baseline"]["max_age_days"], 0)

    def test_profile_one_is_not_rewritten_or_extended(self):
        profile = self.profile(1)
        original = copy.deepcopy(profile)
        result = resolve(profile)
        self.assertEqual(profile, original)
        self.assertNotIn("retention", result)

    def test_retention_unknown_negative_boolean_and_auto_delete_refuse(self):
        mutations = [lambda p: p["retention"].update(keep_latest_per_group=-1),
                     lambda p: p["retention"].update(keep_latest_per_group=True),
                     lambda p: p["retention"].update(keep_latest_per_group=0),
                     lambda p: p["retention"].update(auto_delete=True),
                     lambda p: p["retention"].update(unknown=1),
                     lambda p: p["baseline"].update(max_age_days=-1)]
        for change in mutations:
            with self.subTest(change=change):
                profile = self.profile()
                change(profile)
                with self.assertRaises(OperatorError):
                    resolve(profile)

    def test_runtime_cli_catalogue_query_is_offline_and_external(self):
        config = self.root / "private.toml"
        config.write_text(dumps_toml(self.profile()), encoding="utf-8")
        result = subprocess.run([sys.executable, str(RUNTIME / "evaluate.py"),
            "list", "--config", str(config)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["records"], [])
        self.assertFalse((self.root / "workspace/catalog.sqlite").exists())
        self.assertFalse((self.root / "workspace/catalog.json").exists())
        self.assertFalse((self.root / "workspace/evidence-roots.jsonl").exists())

    def test_single_family_uses_dated_evaluation_and_external_reports(self):
        from operator_flow import prepare, run_prepared
        from test_opi_eval_operator_fixture import Fixture
        from operator_catalog import Store
        fixture = Fixture(self.root, self.profile())
        old_index = fixture.index.read_bytes()
        with fixture.patches():
            operation = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            result = run_prepared(operation, "baseline", repo=fixture.repo)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(operation.parent.name, "families")
        evaluation = operation.parent.parent
        self.assertEqual(evaluation.parent.parent.parent.name, "evaluations")
        self.assertTrue((evaluation / "record.json").is_file())
        self.assertTrue((evaluation / "reports/report.json").is_file())
        self.assertTrue((evaluation / "reports/report.md").is_file())
        self.assertTrue((evaluation / "reports/report.zh.md").is_file())
        rows = Store(fixture.workspace).list()
        self.assertEqual(len(rows), 1)
        self.assertEqual(fixture.index.read_bytes(), old_index)
        self.assertEqual(rows[0]["status"], "complete")

    def test_finished_baseline_rolls_history_to_latest_three_without_database(self):
        from operator_flow import prepare, run_prepared
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        operations = []
        with fixture.patches():
            for _ in range(5):
                operation = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
                result = run_prepared(operation, "baseline", repo=fixture.repo)
                self.assertEqual(result["status"], "complete")
                operations.append(operation)
        store = Store(fixture.workspace)
        retained = store.list(action="baseline", storage_state="active")
        self.assertEqual({row["root"] for row in retained}, {str(path.parent.parent) for path in operations[-3:]})
        for path in operations[:-3]:
            self.assertFalse((path / "snapshot.json").exists())
        for path in operations[-3:]:
            self.assertTrue((path / "snapshot.json").is_file())
        self.assertTrue((operations[-1].parent.parent / "retention-handoff.json").is_file())
        self.assertFalse((fixture.workspace / "catalog.sqlite").exists())
        self.assertFalse((fixture.workspace / "catalog.json").exists())

    def test_finished_result_stays_leased_through_retention_publication(self):
        import operator_flow
        import operator_lifecycle
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        checked = []
        with fixture.patches():
            operation = operator_flow.prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            evaluation = operation.parent.parent
            store = Store(fixture.workspace)
            original_apply = operator_lifecycle.apply_retention
            original_write = operator_flow.write_json
            def assert_leased(stage):
                with self.assertRaises(OperatorError):
                    with store.lease(evaluation.name):
                        self.fail("current result lease was released before " + stage)
                checked.append(stage)
            def retained(*args, **kwargs):
                assert_leased("retention")
                return original_apply(*args, **kwargs)
            def written(path, value):
                if Path(path).name == "retention-handoff.json":
                    assert_leased("summary publication")
                return original_write(path, value)
            with patch("operator_lifecycle.apply_retention", side_effect=retained), patch("operator_flow.write_json", side_effect=written):
                result = operator_flow.run_prepared(operation, "baseline", repo=fixture.repo)
        self.assertEqual(result["retention_status"], "applied")
        self.assertEqual(checked, ["retention", "summary publication"])

    def test_rolling_history_reports_pinned_excess_and_cleans_it_after_unpin(self):
        from operator_flow import prepare, run_prepared
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            first = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            run_prepared(first, "baseline", repo=fixture.repo)
            store = Store(fixture.workspace)
            identity = first.parent.parent.name
            store.event(identity, "pin", reason="synthetic retained evidence")
            for _ in range(3):
                operation = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
                result = run_prepared(operation, "baseline", repo=fixture.repo)
            self.assertEqual(result["retention_status"], "protected_excess")
            summary = json.loads(Path(result["retention"]["path"]).read_text())
            self.assertTrue(any(item["id"] == identity for item in summary["remaining_excess"]))
            self.assertTrue((first / "snapshot.json").is_file())
            store.event(identity, "unpin", reason="synthetic review complete")
            operation = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            result = run_prepared(operation, "baseline", repo=fixture.repo)
        self.assertEqual(result["retention_status"], "applied")
        self.assertEqual(len(store.list(action="baseline", storage_state="active")), 3)
        self.assertFalse((first / "snapshot.json").exists())

    def test_all_families_share_one_directory_and_one_live_deadline(self):
        from operator_flow import prepare, run_prepared
        from operator_process import BatchDeadline
        from test_opi_eval_operator_fixture import Fixture
        from operator_catalog import Store
        profile = self.profile()
        profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
        fixture = Fixture(self.root, profile)
        with fixture.patches():
            evaluation = prepare(fixture.config, "all", "baseline", repo=fixture.repo)
            with patch("operator_flow.BatchDeadline", wraps=BatchDeadline) as timer:
                result = run_prepared(evaluation, "baseline", repo=fixture.repo)
        self.assertEqual(timer.call_count, 1)
        self.assertEqual(result["status"], "complete")
        self.assertEqual(set(result["families"]), {"tb21", "tb30"})
        self.assertTrue((evaluation / "families/tb21/snapshot.json").is_file())
        self.assertTrue((evaluation / "families/tb30/snapshot.json").is_file())
        self.assertEqual(len(Store(fixture.workspace).list()), 1)

    def test_catalogue_does_not_select_retired_pi_for_new_candidate(self):
        from operator_flow import prepare, run_prepared
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            run_prepared(baseline, "baseline", repo=fixture.repo)
            store = Store(fixture.workspace)
            store.event(store.list()[0]["id"], "retire", reason="operator-selected")
            candidate = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
        binding = json.loads((candidate / "inputs/comparison-input.json").read_text())
        self.assertEqual(binding["status"], "baseline_missing")
        self.assertIsNone(binding["baseline"])
        self.assertFalse((candidate / "runs").exists())

    def test_second_family_drift_refuses_the_entire_group_before_dispatch(self):
        from operator_flow import prepare, run_prepared
        from test_opi_eval_operator_fixture import Fixture
        profile = self.profile()
        profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
        fixture = Fixture(self.root, profile)
        with fixture.patches():
            evaluation = prepare(fixture.config, "all", "baseline", repo=fixture.repo)
            material = evaluation / "families/tb30/material/material.json"
            material.write_text("{}")
            with self.assertRaises(OperatorError):
                run_prepared(evaluation, "baseline", repo=fixture.repo)
        self.assertFalse((evaluation / "families/tb21/runs").exists())
        self.assertFalse((evaluation / "execution-authorization.json").exists())

    def test_single_family_record_cannot_redirect_results_to_another_evaluation(self):
        import operator_flow
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            first = operator_flow.prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            second = operator_flow.prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            (second.parent.parent / "record.json").write_bytes((first.parent.parent / "record.json").read_bytes())
            with self.assertRaises(OperatorError):
                operator_flow.run_prepared(second, "candidate", repo=fixture.repo)
        self.assertFalse((second / "runs").exists())
        self.assertFalse((second / "execution-authorization.json").exists())
        self.assertFalse((first.parent.parent / "result-handoff.json").exists())

    def test_expired_group_budget_retains_second_inventory_without_dispatch(self):
        from operator_flow import prepare, run_prepared
        from test_opi_eval_operator_fixture import Fixture
        profile = self.profile()
        profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
        fixture = Fixture(self.root, profile)
        with fixture.patches():
            evaluation = prepare(fixture.config, "all", "baseline", repo=fixture.repo)
            class Deadline:
                def remaining(self):
                    return 0 if (evaluation / "families/tb21/snapshot.json").exists() else 10
            with patch("operator_flow.BatchDeadline", return_value=Deadline()) as timer:
                result = run_prepared(evaluation, "baseline", repo=fixture.repo)
        self.assertEqual(timer.call_count, 1)
        self.assertEqual(result["status"], "incomplete")
        pending = result["families"]["tb30"]
        self.assertEqual(pending["status"], "not_started")
        self.assertEqual(len(pending["unstarted_trials"]), pending["coverage"]["planned"])
        self.assertFalse((evaluation / "families/tb30/runs").exists())

    def test_cli_preview_and_explicit_application_keep_empty_storage_unchanged(self):
        config = self.root / "private.toml"
        config.write_text(dumps_toml(self.profile()), encoding="utf-8")
        plan = self.root / "review.json"
        command = [sys.executable, str(RUNTIME / "evaluate.py")]
        preview = subprocess.run(command + ["gc", "--config", str(config), "--plan", str(plan)],
                                 capture_output=True, text=True)
        self.assertEqual(preview.returncode, 0, preview.stderr)
        self.assertEqual(json.loads(preview.stdout)["status"], "preview")
        self.assertEqual(json.loads(plan.read_text())["selected"], [])
        application = subprocess.run(command + ["apply-plan", "--config", str(config), "--plan", str(plan)],
                                     capture_output=True, text=True)
        self.assertEqual(application.returncode, 0, application.stderr)
        self.assertEqual(json.loads(application.stdout)["results"], [])
        self.assertFalse((self.root / "workspace/archives").exists())

    def test_group_refusal_preserves_finished_family_and_external_gap_report(self):
        import operator_flow
        from test_opi_eval_operator_fixture import Fixture
        profile = self.profile()
        profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
        fixture = Fixture(self.root, profile)
        native_family = operator_flow._run_family
        def run_family(root, action, deadline):
            if root.name == "tb30":
                raise OperatorError("synthetic pre-dispatch refusal")
            return native_family(root, action, deadline)
        with fixture.patches():
            evaluation = operator_flow.prepare(fixture.config, "all", "baseline", repo=fixture.repo)
            with patch("operator_flow._run_family", side_effect=run_family):
                result = operator_flow.run_prepared(evaluation, "baseline", repo=fixture.repo)
        self.assertEqual(result["status"], "stopped")
        self.assertEqual(result["families"]["tb21"]["status"], "complete")
        self.assertEqual(result["families"]["tb30"]["status"], "stopped")
        report = json.loads((evaluation / "reports/report.json").read_text())
        self.assertIsNone(report["families"]["tb30"]["snapshot"])
        self.assertFalse((evaluation / "families/tb30/runs").exists())

    def test_standalone_compare_registers_source_references_and_holds_input_leases(self):
        import operator_flow
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = operator_flow.prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            operator_flow.run_prepared(baseline, "baseline", repo=fixture.repo)
            candidate = operator_flow.prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            operator_flow.run_prepared(candidate, "candidate", repo=fixture.repo)
            store = Store(fixture.workspace)
            original_verify = operator_flow.verify_snapshot
            checked = []
            def verified(snapshot, *args, **kwargs):
                identity = Path(snapshot["self"]["path"]).parent.parent.parent.name
                with self.assertRaises(OperatorError):
                    with store.lease(identity):
                        self.fail("input was not leased during comparison")
                checked.append(identity)
                return original_verify(snapshot, *args, **kwargs)
            with patch("operator_flow.verify_snapshot", side_effect=verified):
                result = operator_flow.compare(candidate / "snapshot.json", baseline / "snapshot.json", self.root / "comparison")
        self.assertEqual(result["status"], "comparable")
        self.assertEqual(len(checked), 2)
        records = store.list(action="compare")
        self.assertEqual(len(records), 1)
        self.assertEqual(set(records[0]["references"]), {str(candidate / "snapshot.json"), str(baseline / "snapshot.json")})
        self.assertFalse(records[0]["reference_unknown"])
        with fixture.patches(), self.assertRaises(OperatorError):
            operator_flow.compare(candidate / "snapshot.json", baseline / "snapshot.json",
                                  Path(records[0]["root"]) / "families/tb21/foreign-comparison")
        self.assertFalse((Path(records[0]["root"]) / "families/tb21/foreign-comparison").exists())
        store.reindex()
        self.assertEqual(len(store.list(action="compare")), 1)
        self.assertTrue((Path(result["evaluation"]) / "reports/report.json").is_file())

    def test_closed_failed_comparisons_roll_wrappers_without_deleting_caller_outputs(self):
        import operator_flow
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = operator_flow.prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            operator_flow.run_prepared(baseline, "baseline", repo=fixture.repo)
            candidate = operator_flow.prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            operator_flow.run_prepared(candidate, "candidate", repo=fixture.repo)
            outputs = []
            for number in range(4):
                out = self.root / f"failed-comparison-{number}"
                with patch("operator_flow.verify_snapshot", side_effect=OperatorError("synthetic offline refusal")), self.assertRaises(OperatorError):
                    operator_flow.compare(candidate / "snapshot.json", baseline / "snapshot.json", out)
                outputs.append(out)
        store = Store(fixture.workspace)
        rows = store.list(action="compare")
        self.assertEqual(len(rows), 4)
        self.assertEqual(sum(row["storage_state"] == "active" for row in rows), 3)
        self.assertEqual(sum(row["storage_state"] == "purged" for row in rows), 1)
        self.assertTrue(all(row["completed_at"] and row["status"] == "stopped" for row in rows))
        self.assertTrue(all(path.is_dir() for path in outputs))
        self.assertTrue((baseline / "snapshot.json").is_file())
        self.assertTrue((candidate / "snapshot.json").is_file())

    def test_grouped_preparation_failure_records_unstarted_families_without_live_dispatch(self):
        import operator_flow
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        profile = self.profile()
        profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
        fixture = Fixture(self.root, profile)
        (fixture.cache / "agents/pi/package.json").write_text('{"version":"0.84.1"}')
        with fixture.patches(), self.assertRaises(OperatorError):
            operator_flow.prepare(fixture.config, "all", "baseline", repo=fixture.repo)
        row = Store(fixture.workspace).list()[0]
        root = Path(row["root"])
        failure = json.loads((root / "preparation-failure.json").read_text())
        self.assertEqual(failure["schema"], "opi-eval-preparation-failure/1")
        self.assertFalse(failure["live_dispatch"])
        self.assertEqual(failure["families"]["tb21"]["status"], "preparation_failed")
        self.assertEqual(failure["families"]["tb30"]["status"], "not_started")
        self.assertTrue(failure["completed_at"])
        self.assertFalse(row["reference_unknown"])
        self.assertFalse((root / "execution-authorization.json").exists())
        self.assertFalse((root / "families/tb30").exists())

    def test_failed_comparison_with_unsettled_command_remains_protected(self):
        import operator_flow
        from operator_catalog import Store
        from test_opi_eval_operator_fixture import Fixture
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = operator_flow.prepare(fixture.config, "tb21", "baseline", repo=fixture.repo)
            operator_flow.run_prepared(baseline, "baseline", repo=fixture.repo)
            candidate = operator_flow.prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            operator_flow.run_prepared(candidate, "candidate", repo=fixture.repo)
            def unsettled(snapshot, commands, *args, **kwargs):
                operator_flow.write_json(commands.root / "0001-unknown.json", {
                    "started_at": "2026-01-01T00:00:00+00:00", "cleanup": "SIGINT-sent-completion-unknown"})
                raise OperatorError("synthetic unsettled cleanup")
            with patch("operator_flow.verify_snapshot", side_effect=unsettled), self.assertRaises(OperatorError):
                operator_flow.compare(candidate / "snapshot.json", baseline / "snapshot.json", self.root / "unsettled")
            store = Store(fixture.workspace)
            identity = store.list(action="compare")[0]["id"]
            for number in range(3):
                with patch("operator_flow.verify_snapshot", side_effect=OperatorError("synthetic closed refusal")), self.assertRaises(OperatorError):
                    operator_flow.compare(candidate / "snapshot.json", baseline / "snapshot.json", self.root / f"closed-{number}")
        rows = store.list(action="compare", storage_state="active")
        self.assertEqual(len(rows), 4)
        protected = store.show(identity)
        handoff = json.loads((Path(protected["root"]) / "families/tb21/handoff.json").read_text())
        self.assertEqual(handoff["cleanup"], "unknown")
        self.assertTrue((baseline / "snapshot.json").is_file())
        latest = max(rows, key=lambda row: row["completed_at"])
        summary = json.loads((Path(latest["root"]) / "retention-handoff.json").read_text())
        self.assertTrue(any(item["id"] == identity for item in summary["remaining_excess"]))

    def test_cli_rejects_missing_reviewed_plan_without_creating_outputs(self):
        config = self.root / "private.toml"
        config.write_text(dumps_toml(self.profile()), encoding="utf-8")
        result = subprocess.run([sys.executable, str(RUNTIME / "evaluate.py"),
            "apply-plan", "--config", str(config), "--plan", str(self.root / "missing.json")],
            capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Traceback", result.stderr)
        self.assertFalse((self.root / "workspace/archives").exists())

    def test_cli_invalid_control_type_is_a_sanitized_refusal(self):
        config = self.root / "private.toml"
        profile = self.profile()
        profile["model"].pop("credential_env")
        profile["model"]["api_key"] = "synthetic-private-profile-value"
        config.write_text(dumps_toml(profile), encoding="utf-8")
        result = subprocess.run([sys.executable, str(RUNTIME / "evaluate.py"), "resolve", "--config", str(config),
                                 "--set", "model.thinking.level=[]"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertNotIn("Traceback", result.stderr)
        self.assertNotIn(profile["model"]["api_key"], result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
