#!/usr/bin/env python3
"""Offline contracts for the Companion eval operator; no provider calls."""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[4]
SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))


class OperatorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.addCleanup(self.temp.cleanup)

    def profile(self):
        return {
            "schema": "opi-eval-operator/1",
            "model": {"provider": "local", "model": "synthetic", "api": "openai-completions",
                      "base_url": "http://127.0.0.1:4444/v1", "credential_env": "TEST_KEY",
                      "controls": {"context_window": 8192, "max_output_tokens": 4096,
                                   "temperature": "omitted", "top_p": "omitted"},
                      "thinking": {"level": "high", "required": True}},
            "execution": {"trial_retries": 2, "batch_timeout_seconds": 60,
                          "concurrency": 1, "capture_policy": "full"},
            "benchmarks": {"tb21": {"scope": "subset", "task_ids": ["one", "two"],
                                    "repetitions": 2, "seeds": {"one": 17}}},
            "baseline": {"pi_version": "1.0.0", "lookup": "auto"},
            "storage": {"workspace": str(self.root / "workspace"), "cache": str(self.root / "cache")}}

    def test_resolution_is_secret_free_and_applies_explicit_overrides(self):
        from operator_config import resolve
        profile = self.profile()
        profile["model"].pop("credential_env")
        profile["model"]["api_key"] = "synthetic-secret-never-export"
        effective = resolve(profile, ["model.thinking.level=\"off\"", "execution.trial_retries=0"])
        self.assertNotIn("synthetic-secret-never-export", json.dumps(effective))
        self.assertEqual(effective["model"]["thinking"]["level"], "off")
        self.assertEqual(effective["credential"], {"kind": "private-profile"})
        self.assertEqual(effective["execution"]["trial_retries"], 0)

    def test_table_override_cannot_export_inline_credentials(self):
        from operator_config import resolve, dumps_toml, OperatorError
        profile = self.profile()
        override = 'model={api_key="synthetic-canary-secret"}'
        with self.assertRaises(OperatorError):
            resolve(profile, [override])
        config = self.root / "private.toml"
        config.write_text(dumps_toml(profile))
        result = subprocess.run([sys.executable, str(SCRIPTS / "evaluate.py"), "resolve", "--config", str(config), "--set", override], capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("synthetic-canary-secret", result.stdout + result.stderr)

    def test_rejects_unknown_keys_bad_credentials_and_invalid_bounds(self):
        from operator_config import resolve, OperatorError
        mutations = [lambda p: p.update({"mystery": 1}),
                     lambda p: p["model"].update({"api_key": "secret"}),
                     lambda p: p["execution"].update({"trial_retries": -1}),
                     lambda p: p["execution"].update({"concurrency": True}),
                     lambda p: p["model"]["thinking"].update({"level": "imaginary"}),
                     lambda p: p["benchmarks"]["tb21"].update({"scope": "full"}),
                     lambda p: p["model"].update({"base_url": "https://user:secret@example.org"})]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                p = self.profile()
                mutate(p)
                with self.assertRaises(OperatorError):
                    resolve(p)

    def test_init_exclusive_and_required_thinking_projection(self):
        from operator_config import init_profile, resolve, experiment, OperatorError
        target = self.root / ".opi/eval.toml"
        init_profile(target)
        with self.assertRaises(OperatorError):
            init_profile(target)
        effective = resolve(self.profile())
        document = experiment(effective, "tb21", "opi", "0.8.2", "op-test", self.root / "cache", self.root)
        self.assertEqual(document["model_controls"]["reasoning"], "high")
        self.assertIn("reasoning", document["model_controls"]["required"])
        self.assertEqual(len(document["subjects"]), 1)
        self.assertNotIn("edges", document)

    def test_toml_round_trip_quotes_paths_and_seeds(self):
        from operator_config import dumps_toml, resolve, experiment
        import tomllib
        p = resolve(self.profile())
        doc = experiment(p, "tb21", "pi", "1.0.0", "test", self.root / "cache", self.root)
        doc["selection"]["seeds"] = {"a.b\"c": 42}
        self.assertEqual(tomllib.loads(dumps_toml(doc)), doc)

    def test_thinking_levels_resolve_and_project_in_both_profile_schemas(self):
        from operator_config import TEMPLATE, load, resolve, experiment
        for profile in (self.profile(), load(TEMPLATE)):
            for level in ("off", "minimal", "low", "medium", "high", "xhigh", "max", "omitted"):
                with self.subTest(schema=profile["schema"], level=level):
                    p = copy.deepcopy(profile)
                    p["model"]["thinking"] = {"level": level, "required": level != "omitted"}
                    effective = resolve(p)
                    document = experiment(effective, "tb21", "pi", "1.0.4", "test", self.root / "cache", self.root)
                    self.assertEqual(effective["model"]["thinking"]["level"], level)
                    self.assertEqual(document["model_controls"]["reasoning"], level)
                    self.assertEqual("reasoning" in document["model_controls"]["required"], level != "omitted")

    def test_invalid_enum_values_raise_operator_error_before_storage_creation(self):
        from operator_config import resolve, OperatorError
        for name in ("schema", "model.api", "model.thinking.level", "execution.capture_policy",
                     "benchmarks.tb21.scope", "baseline.lookup"):
            for value in ("unknown-choice", "MAX", "", 42, True, [], {}, None):
                with self.subTest(name=name, value=value):
                    p = self.profile()
                    components = name.split(".")
                    target = p
                    for component in components[:-1]:
                        target = target[component]
                    target[components[-1]] = value
                    with self.assertRaises(OperatorError):
                        resolve(p)
        self.assertFalse((self.root / "workspace").exists())
        self.assertFalse((self.root / "cache").exists())

    def test_concurrency_matches_companion_u32_bound(self):
        from operator_config import TEMPLATE, load, resolve, experiment, OperatorError
        for profile in (self.profile(), load(TEMPLATE)):
            with self.subTest(schema=profile["schema"]):
                profile["execution"]["concurrency"] = 2**32 - 1
                effective = resolve(profile)
                document = experiment(effective, "tb21", "pi", "1.0.4", "test", self.root / "cache", self.root)
                self.assertEqual(document["execution"]["concurrency"], 2**32 - 1)
                profile["execution"]["concurrency"] = 2**32
                with self.assertRaises(OperatorError):
                    resolve(profile)
        self.assertFalse((self.root / "workspace").exists())
        self.assertFalse((self.root / "cache").exists())

    def test_sampling_values_keep_companion_float_type_through_toml_projection(self):
        from operator_config import TEMPLATE, load, resolve, experiment, dumps_toml
        import tomllib
        for profile in (self.profile(), load(TEMPLATE)):
            for temperature, top_p in ((0, 1), (2, 0), (0.25, 0.9), ("omitted", "unknown"), ("unknown", "omitted")):
                with self.subTest(schema=profile["schema"], temperature=temperature, top_p=top_p):
                    p = copy.deepcopy(profile)
                    p["model"]["controls"].update(temperature=temperature, top_p=top_p)
                    effective = resolve(p)
                    document = experiment(effective, "tb21", "pi", "1.0.4", "test", self.root / "cache", self.root)
                    parsed = tomllib.loads(dumps_toml(document))
                    for controls in (effective["model"]["controls"], document["model_controls"], parsed["model_controls"]):
                        for name, requested in (("temperature", temperature), ("top_p", top_p)):
                            expected = requested if isinstance(requested, str) else float(requested)
                            self.assertEqual(controls[name], expected)
                            self.assertIs(type(controls[name]), type(expected))

    def test_cli_resolve_accepts_max_and_refuses_invalid_thinking(self):
        from operator_config import TEMPLATE, load, dumps_toml
        config = self.root / "private.toml"
        config.write_text(dumps_toml(load(TEMPLATE)))
        command = [sys.executable, str(SCRIPTS / "evaluate.py"), "resolve", "--config", str(config), "--set"]
        result = subprocess.run([*command, 'model.thinking.level="max"'], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["model"]["thinking"], {"level": "max", "required": True})
        for value in ('"ultra"', '["max"]'):
            with self.subTest(value=value):
                result = subprocess.run([*command, "model.thinking.level=" + value], capture_output=True, text=True)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertIn("unknown thinking level", result.stderr)

    def test_tasks_selection_translates_to_subset_overrides_and_validates(self):
        import evaluate
        from operator_process import OperatorError
        class Arguments:
            bench = "tb21"
            tasks = "build-pmars, build-pov-ray"
        self.assertEqual(evaluate.tasks_selection(Arguments()),
                         ['benchmarks.tb21.scope="subset"',
                          'benchmarks.tb21.task_ids=["build-pmars", "build-pov-ray"]'])
        for bench, tasks in (("all", "build-pmars"), ("tb21", ""), ("tb21", "a,,b"),
                             ("tb21", "a,a"), (None, "build-pmars")):
            with self.subTest(bench=bench, tasks=tasks):
                class Rejected:
                    pass
                Rejected.bench, Rejected.tasks = bench, tasks
                with self.assertRaises(OperatorError):
                    evaluate.tasks_selection(Rejected())

    def test_cli_tasks_prepare_refuses_without_single_family(self):
        from operator_config import TEMPLATE, load, dumps_toml
        config = self.root / "private.toml"
        config.write_text(dumps_toml(load(TEMPLATE)))
        command = [sys.executable, str(SCRIPTS / "evaluate.py"), "baseline", "--config", str(config)]
        result = subprocess.run([*command, "--bench", "all", "--tasks", "build-pmars"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 2)
        self.assertIn("exactly one benchmark family", result.stderr)

    def test_retry_is_precise_valid_zero_never_retries_and_breach_stops(self):
        from operator_evidence import retry_trials, OperatorError
        planned = [{"id": "one", "task": "a", "subject": "opi", "group": "a-rep1"},
                   {"id": "two", "task": "b", "subject": "opi", "group": "b-rep2"},
                   {"id": "three", "task": "c", "subject": "opi", "group": "c-rep1"}]
        report = {"trials": [{"id": "one", "outcome": {"class": "graded"},
                               "headline": {"reward": {"state": "measured", "value": 0}}},
                              {"id": "two", "outcome": {"class": "invalid", "reason": "provider-access"}}]}
        self.assertEqual(retry_trials(planned, report, 1)[0]["task"], "b")
        self.assertEqual(len(retry_trials(planned, report, 1)), 1)
        report["trials"][1]["outcome"]["reason"] = "authority-boundary"
        with self.assertRaises(OperatorError):
            retry_trials(planned, report, 1)
        for reason in ("owned-resource-cleanup-unconfirmed", "native-command-termination-unconfirmed",
                       "credential-material-removal-failed", "partial-capture-unconfirmed"):
            report["trials"][1]["outcome"]["reason"] = reason
            with self.subTest(reason=reason), self.assertRaises(OperatorError):
                retry_trials(planned, report, 1)

    def test_shared_deadline_never_resets(self):
        from operator_process import BatchDeadline
        clock = [10.0]
        deadline = BatchDeadline(60, clock=lambda: clock[0])
        self.assertEqual(deadline.remaining(), 60)
        clock[0] = 69
        self.assertEqual(deadline.remaining(), 1)
        clock[0] = 71
        self.assertEqual(deadline.remaining(), 0)

    def test_export_results_writes_conclusions_and_windows_evidence(self):
        import operator_flow
        evaluation = self.root / "evaluations/2026/10/baseline-x"
        (evaluation / "families/tb21/checks/attempt-0").mkdir(parents=True)
        (evaluation / "reports").mkdir(parents=True)
        write = lambda path, value: path.write_text(json.dumps(value), encoding="utf-8")
        write(evaluation / "record.json", {"action": "baseline", "created_at": "2026-10-12T00:00:00Z",
                                           "families": {"tb21": str(evaluation / "families/tb21")}})
        write(evaluation / "result-handoff.json", {"status": "stopped", "completed_at": "2026-10-12T00:00:00Z",
                                                    "families": {"tb21": {"status": "stopped", "coverage": {"planned": 6, "valid": 5}}}})
        write(evaluation / "families/tb21/snapshot.json", {"product": "pi", "version": "1.0.4", "stopped_reason": "gap",
            "slots": [
                {"task": "task-a", "repetition": 1, "valid": True, "reward": 1.0, "attempts": [{"outcome": {"class": "graded"}}]},
                {"task": "task-b", "repetition": 1, "valid": False, "reward": None, "attempts": [{"outcome": {"class": "invalid", "reason": "native-capture-refused"}}]},
            ]})
        write(evaluation / "families/tb21/checks/attempt-0/report-a.json", {"manifest_digest": "m", "integrity_digest": "i"})
        write(evaluation / "reports/report.json", {"schema": "opi-eval-evaluation-report/1"})
        (evaluation / "reports/report.md").write_text("report", encoding="utf-8")
        exports = self.root / "exports"
        profile = {"storage": {"exports": str(exports)}}
        outcome = operator_flow._export_results(evaluation, profile, self.root)
        self.assertEqual(outcome["status"], "completed")
        stem = "2026-10-12_baseline-tb21_baseline-x"
        for suffix in ("", ".zh"):
            conclusion = self.root / "docs/eval" / (stem + suffix + ".md")
            self.assertTrue(conclusion.is_file(), conclusion)
            text = conclusion.read_text(encoding="utf-8")
            self.assertIn("manifest `m", text)
            self.assertIn("逐任务结果" if suffix else "Per-task results", text)
            self.assertIn("task-a", text)
            self.assertIn("native-task-success = 1/1" if not suffix else "native-task-success = 1/1", text)
        copied = exports / "baseline-x"
        self.assertTrue((copied / "result-handoff.json").is_file())
        self.assertTrue((copied / "reports/report.md").is_file())
        self.assertTrue((copied / "families/tb21/checks/attempt-0/report-a.json").is_file())
        receipt = json.loads((evaluation / "export-handoff.json").read_text(encoding="utf-8"))
        self.assertEqual(receipt["schema"], "opi-eval-export-handoff/1")
        self.assertEqual(len(receipt["conclusions"]), 2)
        self.assertTrue(all(not entry.startswith("/") for entry in receipt["conclusions"] + receipt["evidence"]))
        self.assertEqual(operator_flow._windows_view(Path("/mnt/d/a/b")), "D:\\a\\b")
        self.assertIsNone(operator_flow._windows_view(Path("/home/luiz/a")))

    def test_export_failure_records_gap_and_keeps_results(self):
        import operator_flow
        evaluation = self.root / "evaluations/2026/10/baseline-y"
        evaluation.mkdir(parents=True)
        outcome = operator_flow._export_results(evaluation, {}, self.root)
        self.assertEqual(outcome["status"], "refused")
        self.assertIn("export refused", outcome["gap"])

    def test_resolve_accepts_optional_disjoint_exports_root(self):
        from operator_config import TEMPLATE, load, resolve, OperatorError
        profile = load(TEMPLATE)
        profile["storage"]["exports"] = str(self.root / "exports")
        resolved = resolve(profile)
        self.assertEqual(resolved["storage"]["exports"], str(self.root / "exports"))
        profile["storage"]["exports"] = str(Path(profile["storage"]["workspace"]) / "inside")
        with self.assertRaises(OperatorError):
            resolve(profile)

    def test_subset_export_merges_into_matching_full_evaluation(self):
        import operator_flow
        base = self.root / "evaluations/2026/10"
        primary, subset = base / "baseline-full", base / "baseline-sub"
        for evaluation, scope, task_rows in ((primary, "full", [("task-a", False, None, "native-capture-refused"), ("task-b", True, 1.0, None)]),
                                              (subset, "subset", [("task-a", True, 1.0, None)])):
            (evaluation / "families/tb21/checks/attempt-0").mkdir(parents=True)
            (evaluation / "families/tb21/inputs").mkdir(parents=True)
            (evaluation / "reports").mkdir(parents=True)
            write = lambda path, value: path.write_text(json.dumps(value), encoding="utf-8")
            write(evaluation / "record.json", {"action": "baseline", "created_at": "2026-10-12T00:00:00Z",
                                               "families": {"tb21": str(evaluation / "families/tb21")}})
            write(evaluation / "families/tb21/inputs/effective.json", {"benchmarks": {"tb21": {"scope": scope}}})
            common = {"connection": {"provider": "zhipu", "model": "m", "api": "a", "base_url": "u"},
                      "controls": {"reasoning": "high"}, "corpus": {"source_commit": "c"}}
            slots = []
            for task, valid, reward, reason in task_rows:
                outcome = {"class": "graded"} if valid else {"class": "invalid", "reason": reason}
                slots.append({"task": task, "repetition": 1, "valid": valid, "reward": reward,
                              "attempts": [{"outcome": outcome}]})
            write(evaluation / "families/tb21/snapshot.json", {"product": "pi", "version": "1.0.4",
                "semantic": {"common": common}, "slots": slots,
                "companion": {"sha256": "x" * 64}})
            write(evaluation / "families/tb21/checks/attempt-0/report-a.json", {"manifest_digest": "m", "integrity_digest": "i"})
            write(evaluation / "result-handoff.json", {"status": "stopped", "completed_at": "2026-10-12T00:00:00Z",
                "families": {"tb21": {"status": "stopped", "coverage": {"planned": len(task_rows)}}}})
        class Store:
            def list(self):
                return [{"root": str(primary)}, {"root": str(subset)}]
        profile = {"storage": {}}
        outcome = operator_flow._export_results(subset, profile, self.root, store=Store())
        self.assertEqual(outcome["merged_into"], primary.name)
        merged = self.root / "docs/eval/2026-10-12_baseline-tb21_baseline-full.md"
        self.assertTrue(merged.is_file())
        text = merged.read_text(encoding="utf-8")
        self.assertIn("Metric (merged, supplements replace)", text)
        self.assertIn("Supplement sealed tree", text)
        self.assertIn("baseline-sub", text)
        self.assertIn("| 1 | task-a | invalid: agent-timeout (capture refused) | pass | pass | 1.0 |", text)
        self.assertFalse((self.root / "docs/eval/2026-10-12_baseline-tb21_baseline-sub.md").exists())
        self.assertTrue((self.root / "docs/eval/2026-10-12_baseline-tb21_baseline-full.zh.md").is_file())
        # differing controls never merge: the subset writes its own report
        drift = base / "baseline-drift"
        (drift / "families/tb21").mkdir(parents=True)
        (drift / "record.json").write_text(json.dumps({"action": "baseline", "created_at": "2026-10-12T00:00:01Z",
            "families": {"tb21": str(drift / "families/tb21")}}), encoding="utf-8")
        (drift / "families/tb21/inputs").mkdir(parents=True)
        (drift / "families/tb21/inputs/effective.json").write_text(json.dumps({"benchmarks": {"tb21": {"scope": "subset"}}}), encoding="utf-8")
        snapshot = json.loads((primary / "families/tb21/snapshot.json").read_text(encoding="utf-8"))
        snapshot["semantic"]["common"]["controls"]["reasoning"] = "max"
        (drift / "families/tb21/snapshot.json").write_text(json.dumps(snapshot), encoding="utf-8")
        (drift / "result-handoff.json").write_text(json.dumps({"status": "stopped", "families": {}}), encoding="utf-8")
        class DriftStore:
            def list(self):
                return [{"root": str(primary)}, {"root": str(drift)}]
        outcome = operator_flow._export_results(drift, profile, self.root, store=DriftStore())
        self.assertIsNone(outcome["merged_into"])
        self.assertTrue(any(entry.endswith("baseline-drift.md") for entry in outcome["conclusions"]))

    def test_exclusive_artifacts(self):
        from operator_process import write_json, OperatorError
        target = self.root / "manifest.json"
        write_json(target, {"value": 1})
        with self.assertRaises(OperatorError):
            write_json(target, {"value": 2})
        self.assertEqual(json.loads(target.read_text()), {"value": 1})

    def test_semantic_match_ignores_paths_and_agent_but_checks_task_and_controls(self):
        from operator_evidence import semantic_identity
        profile = self.profile()
        material = {"benchmarks": {"terminal-bench-2.1": {
            "corpus": {"root": "/x", "source_commit": "abc", "tasks_tree": "tree"},
            "tasks": {"one": {"task_package": "/x/one", "profile": {"path": "/x/p", "sha256": "p"},
                               "task_package_manifest_sha256": "task"}},
            "verifier_executable": {"path": "/x/uv", "sha256": "uv"},
            "runner_project": {"path": "/x/runner", "source_manifest_sha256": "runner"}}}}
        plan = {"experiment": {"benchmark": {"name": "terminal-bench", "revision": "2.1", "dataset": "tb21"},
                               "model_controls": {"reasoning": "high"}, "execution": {},
                               "connection": "model", "connections": [{"id":"model", **{key: profile["model"][key] for key in ("provider", "model", "api", "base_url")}}],
                               "storage":{"capture_policy":"full"}},
                "selection": {"seeds": {}}, "corpus": [{"id": "one", "requirements": {"cpus": 1}}]}
        a = semantic_identity(profile, "tb21", material, plan)
        material["benchmarks"]["terminal-bench-2.1"]["corpus"]["root"] = "/new"
        self.assertEqual(a, semantic_identity(profile, "tb21", material, plan))
        plan["experiment"]["model_controls"]["reasoning"] = "off"
        self.assertNotEqual(a, semantic_identity(profile, "tb21", material, plan))

    def test_latest_eligible_not_best_score_and_missing_is_standalone(self):
        from operator_evidence import choose_baseline, compare_snapshots
        candidates = [{"id": "a", "completed_at": "2026-01-01", "product": "pi", "complete": True,
                       "companion":{"sha256":"a"*64}, "version":"1.0.0", "pi_version":"1.0.0",
                       "semantic": {"common": {"model": "x", "admitted": True}, "tasks": {"one": {"admitted": True}}}, "slots": []},
                      {"id": "b", "completed_at": "2026-02-01", "product": "pi", "complete": True,
                       "companion":{"sha256":"a"*64}, "version":"1.0.0", "pi_version":"1.0.0",
                       "semantic": {"common": {"model": "x", "admitted": True}, "tasks": {"one": {"admitted": True}}}, "slots": []}]
        target = copy.deepcopy(candidates[0])
        target["product"] = "opi"
        self.assertEqual(choose_baseline(candidates, target)["id"], "b")
        candidates[0]["completed_at"] = "2020-01-02T00:00:00+14:00"
        candidates[1]["completed_at"] = "2020-01-01T23:00:00+00:00"
        self.assertEqual(choose_baseline(candidates, target)["id"], "b")
        self.assertEqual(compare_snapshots(target, None)["status"], "baseline_missing")
        self.assertIsNone(compare_snapshots(target, None)["paired_reward_delta"])

    def test_partial_pairing_keeps_missing_slots_and_unknown_metrics(self):
        from operator_evidence import compare_snapshots
        base = {"id": "pi", "product": "pi", "complete": True,
                "companion":{"sha256":"a"*64}, "version":"1.0.0", "pi_version":"1.0.0",
                "semantic": {"common": {"admitted": True}, "tasks": {"a": {"admitted": True}, "b": {"admitted": True}}},
                "slots": [{"task": "a", "repetition": 1, "seed": None, "reward": 0, "valid": True},
                          {"task": "b", "repetition": 1, "seed": None, "reward": 1, "valid": True}]}
        cand = copy.deepcopy(base)
        cand["id"] = "opi"
        cand["product"] = "opi"
        cand["slots"] = [{"task": "a", "repetition": 1, "seed": None, "reward": 1, "valid": True}]
        result = compare_snapshots(cand, base)
        self.assertEqual(result["paired_reward_delta"], 1)
        self.assertEqual(result["coverage"]["missing_candidate"], 1)
        self.assertEqual(result["efficiency"]["state"], "unknown")

    def test_cli_invalid_profile_refuses_without_secret_output(self):
        config = self.root / "bad.toml"
        config.write_text('schema="opi-eval-operator/1"\n[model]\napi_key="synthetic-secret"\n')
        result = subprocess.run([sys.executable, str(SCRIPTS / "evaluate.py"), "resolve", "--config", str(config)],
                                capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("synthetic-secret", result.stdout + result.stderr)

    def test_overlapping_storage_is_rejected_before_any_creation(self):
        from operator_config import resolve, OperatorError
        p = self.profile()
        for path in (p["storage"]["workspace"], str(Path(p["storage"]["workspace"]) / "child"), str(self.root)):
            p["storage"]["cache"] = path
            with self.assertRaises(OperatorError):
                resolve(p)
        self.assertFalse((self.root / "workspace").exists())

    def test_repetitions_reject_u32_overflow_and_large_plan_before_dispatch(self):
        from operator_config import resolve, OperatorError
        p = self.profile()
        for repetition in (2**32, 2**32-1, 10000):
            p["benchmarks"]["tb21"]["repetitions"] = repetition
            with self.subTest(repetition=repetition), self.assertRaises(OperatorError):
                resolve(p)
        self.assertFalse((self.root / "workspace").exists())

    def test_actual_native_reward_token_and_attribution(self):
        from operator_evidence import logical_slots
        planned = {"trials": [{"id": "trial-opi-task-retry-name-r2", "task": "task-retry-name", "subject": "opi", "group": "task-retry-name-r2"}],
                   "selection": {"seeds": {}}}
        rows = [{**planned["trials"][0], "outcome": {"class": "graded", "source": "settlement"},
                 "headline": {"reward": "known:1.0(native-trial-result)",
                              "native_source": {"artifact": "native/trial-result.json", "digest": "a" * 64}}}]
        report = {"root": "/synthetic", "report_pin": {"path": "/report", "sha256": "r"}, "report": {"trials": rows}}
        slots = logical_slots(planned, [report])
        self.assertTrue(slots[0]["valid"])
        self.assertEqual(slots[0]["reward"], 1)
        rows[0]["task"] = "wrong-task"
        self.assertFalse(logical_slots(planned, [report])[0]["valid"])

    def test_secret_environment_exclusion_and_bounded_capture(self):
        from operator_process import Commands, OperatorError
        command = Commands(self.root / "commands", excluded_env=["CUSTOM_AUTH"], capture_cap=1024)
        with patch.dict(os.environ, {"CUSTOM_AUTH": "synthetic-secret"}):
            result = command.run([sys.executable, "-c", "import os;print(os.environ.get('CUSTOM_AUTH', 'absent'))"], self.root)
        self.assertEqual(result.stdout.strip(), "absent")
        with self.assertRaises(OperatorError):
            command.run([sys.executable, "-c", "print('x'*2048)"], self.root)
        records = [json.loads(path.read_text()) for path in sorted((self.root / "commands").glob("*.json"))]
        self.assertTrue(records[-1]["capture_overflow"])

    def test_explicit_incompatible_baseline_cannot_bypass_matching(self):
        from operator_evidence import compare_snapshots
        base = {"id": "pi", "product": "pi", "complete": True, "version": "1.0.0",
                "companion": {"sha256": "old"}, "semantic": {"common": {"admitted": True}, "tasks": {"a": {"admitted": True}}}, "slots": []}
        cand = copy.deepcopy(base)
        cand.update(product="opi", pi_version="1.0.0", companion={"sha256": "new"})
        self.assertEqual(compare_snapshots(cand, base)["status"], "incomparable")

    def test_current_dirty_build_is_frozen_and_failed_build_never_reuses_it(self):
        from operator_flow import build
        from operator_process import Commands, OperatorError, sha
        repo = self.root / "build-repo"
        (repo / "crates/sample/src").mkdir(parents=True)
        source = repo / "crates/sample/src/main.rs"
        source.write_text("current dirty source")
        (repo / "Cargo.lock").write_text("locked dependencies")
        (repo / "Cargo.toml").write_text("workspace")
        artifact = self.root / "compiler-binary"
        artifact.write_text("new compiled executable")
        calls = []
        def command(argv, cwd, **kwargs):
            argv = [str(value) for value in argv]
            if argv[:2] == ["git", "ls-files"]:
                stdout = "Cargo.lock\0Cargo.toml\0crates/sample/src/main.rs\0"
            elif argv[:2] == ["git", "rev-parse"]:
                stdout = "head"
            elif argv[:2] == ["git", "status"]:
                stdout = " M crates/sample/src/main.rs\n"
            elif argv == ["rustc", "-Vv"]:
                stdout = "host: x86_64-unknown-linux-gnu"
            else:
                stdout = "synthetic help"
            return subprocess.CompletedProcess(argv, 0, stdout, "")
        def compiler(commands, root, package, binary, env, target):
            calls.append(binary)
            return artifact
        operation = self.root / "build-operation"
        (operation / "cache").mkdir(parents=True)
        commands = Commands(operation / "commands")
        with patch.object(commands, "run", side_effect=command), patch("operator_flow.cargo_build", side_effect=compiler), patch.dict(os.environ, {"CARGO_TARGET_DIR":str(self.root / "shared-cargo")}):
            identity = build(commands, operation, "opi", repo=repo)
        self.assertEqual(calls, ["opi-eval", "opi"])
        self.assertEqual((operation / "builds/source/crates/sample/src/main.rs").read_text(), "current dirty source")
        self.assertEqual(identity["opi"]["sha256"], sha(artifact))
        self.assertTrue(json.loads((operation / "builds/source-before.json").read_text())["dirty"])
        failed = self.root / "failed-build"
        (failed / "cache").mkdir(parents=True)
        failed_commands = Commands(failed / "commands")
        with patch.object(failed_commands, "run", side_effect=command), patch("operator_flow.cargo_build", side_effect=OperatorError("synthetic locked build failure")), patch.dict(os.environ, {"CARGO_TARGET_DIR":str(self.root / "shared-cargo")}):
            with self.assertRaises(OperatorError):
                build(failed_commands, failed, "opi", repo=repo)
        self.assertFalse((failed / "cache/agents/opi/opi").exists())
        changed = self.root / "changed-build"
        (changed / "cache").mkdir(parents=True)
        changed_commands = Commands(changed / "commands")
        def changing_compiler(*args):
            source.write_text("concurrent source change")
            return artifact
        with patch.object(changed_commands, "run", side_effect=command), patch("operator_flow.cargo_build", side_effect=changing_compiler), patch.dict(os.environ, {"CARGO_TARGET_DIR":str(self.root / "shared-cargo")}):
            with self.assertRaises(OperatorError):
                build(changed_commands, changed, "opi", repo=repo)
        self.assertFalse((changed / "builds/identity.json").exists())

    def test_source_links_are_internal_frozen_and_order_independent(self):
        from operator_flow import build
        from operator_process import Commands
        for case, target_name, link_name, absolute in (
                ("absolute", "atarget.rs", "zlink.rs", True),
                ("link-before-target", "ztarget.rs", "alink.rs", False)):
            with self.subTest(case=case):
                repo = self.root / case / "repo"
                source_dir = repo / "crates/sample/src"
                source_dir.mkdir(parents=True)
                source = source_dir / target_name
                source.write_text("original frozen bytes")
                link = source_dir / link_name
                try:
                    link.symlink_to(source if absolute else target_name)
                except OSError as error:
                    self.skipTest(f"file symlinks unavailable: {error}")
                (repo / "Cargo.lock").write_text("locked dependencies")
                (repo / "Cargo.toml").write_text("workspace")
                artifact = self.root / case / "compiler-binary"
                artifact.write_text("compiled executable")
                def command(argv, cwd, **kwargs):
                    argv = [str(value) for value in argv]
                    if argv[:2] == ["git", "ls-files"]:
                        stdout = "\0".join(["Cargo.lock", "Cargo.toml",
                            f"crates/sample/src/{target_name}", f"crates/sample/src/{link_name}", ""])
                    elif argv[:2] == ["git", "rev-parse"]:
                        stdout = "head"
                    elif argv[:2] == ["git", "status"]:
                        stdout = ""
                    else:
                        stdout = "synthetic help"
                    return subprocess.CompletedProcess(argv, 0, stdout, "")
                operation = self.root / case / "operation"
                (operation / "cache").mkdir(parents=True)
                commands = Commands(operation / "commands")
                with patch.object(commands, "run", side_effect=command), patch("operator_flow.cargo_build", return_value=artifact), patch.dict(os.environ, {"CARGO_TARGET_DIR": str(self.root / "shared-cargo")}):
                    build(commands, operation, "opi", repo=repo)
                archive = operation / "builds/source"
                frozen_link = archive / "crates/sample/src" / link_name
                self.assertTrue(frozen_link.is_symlink())
                self.assertFalse(Path(os.readlink(frozen_link)).is_absolute())
                self.assertEqual(frozen_link.resolve(), archive / "crates/sample/src" / target_name)
                source.write_text("later live checkout bytes")
                self.assertEqual(frozen_link.read_text(), "original frozen bytes")


class NativeFlowTests(OperatorTests):
    """Real local subprocesses emit current native schemas; providers/builds are inert."""
    def test_prepare_run_exact_retry_compare_and_tamper_refusal(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare, run_prepared, compare
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo, index=fixture.index)
            pi = run_prepared(baseline, "baseline", repo=fixture.repo, index=fixture.index)
            self.assertEqual(pi["status"], "complete")
            candidate = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo, index=fixture.index)
            input_path = json.loads((candidate / "prepared.json").read_text())["comparison_input"]["path"]
            self.assertEqual(json.loads(Path(input_path).read_text())["status"], "selected")
            opi = run_prepared(candidate, "candidate", repo=fixture.repo, index=fixture.index)
            self.assertEqual(opi["status"], "complete")
            snapshot = json.loads((candidate / "snapshot.json").read_text())
            self.assertEqual(len(snapshot["attempts"]), 2)
            self.assertEqual(len(snapshot["attempts"][1]["trials"]), 1)
            self.assertEqual(snapshot["attempts"][1]["trials"][0]["task"], "two")
            result = compare(candidate / "snapshot.json", baseline / "snapshot.json", self.root / "comparison", index=fixture.index)
            self.assertEqual(result["status"], "comparable")
            self.assertEqual(result["paired_reward_delta"], 0)
            snapshot["slots"][0]["reward"] = 1
            (candidate / "snapshot.json").write_text(json.dumps(snapshot))
            from operator_process import OperatorError
            with self.assertRaises(OperatorError):
                compare(candidate / "snapshot.json", baseline / "snapshot.json", self.root / "tampered", index=fixture.index)

    def test_wrong_pi_cache_version_and_failure_handoff(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare
        from operator_process import OperatorError
        fixture = Fixture(self.root, self.profile())
        meta = fixture.cache / "agents/pi/package.json"
        meta.write_text('{"version":"0.84.1"}')
        with fixture.patches(), self.assertRaises(OperatorError):
            prepare(fixture.config, "tb21", "baseline", repo=fixture.repo, index=fixture.index)
        roots = [path.parent for path in (fixture.workspace / "evaluations").glob("*/*/*/record.json")]
        self.assertEqual(len(roots), 1)
        self.assertEqual(json.loads((roots[0] / "handoff.json").read_text())["status"], "preparation_failed")
        self.assertFalse((roots[0] / "runs").exists())

    def test_report_failure_preserves_dispatched_attempt_and_original_zero(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare, run_prepared
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            candidate = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo, index=fixture.index)
            (candidate / "fail-report").write_text("synthetic refusal")
            result = run_prepared(candidate, "candidate", repo=fixture.repo, index=fixture.index)
            self.assertEqual(result["status"], "stopped")
            self.assertEqual(result["coverage"]["dispatched_attempt_runs"], 1)
            self.assertEqual(result["coverage"]["unverified_attempt_runs"], 1)
            self.assertTrue((candidate / "runs/attempt-0/run-control.json").is_file())
            self.assertEqual(json.loads((candidate / "snapshot.json").read_text())["attempts"][0]["verification"], "pending")
            self.assertTrue(all(slot["state"] == "verification_unknown" for slot in json.loads((candidate / "snapshot.json").read_text())["slots"]))

    def test_compare_refuses_nested_original_destination_before_creating_it(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare, run_prepared, compare
        from operator_process import OperatorError
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            candidate = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo, index=fixture.index)
            run_prepared(candidate, "candidate", repo=fixture.repo, index=fixture.index)
            destination = candidate / "runs/attempt-0/new-comparison"
            with self.assertRaises(OperatorError):
                compare(candidate / "snapshot.json", None, destination, index=fixture.index)
            self.assertFalse(destination.exists())

    def test_corrupt_automatic_baseline_still_prepares_standalone_candidate(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare, run_prepared
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo, index=fixture.index)
            run_prepared(baseline, "baseline", repo=fixture.repo, index=fixture.index)
            (baseline / "runs/attempt-0/native-report.json").write_text("corrupt original")
            candidate = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo, index=fixture.index)
            inputs = json.loads((candidate / "inputs/comparison-input.json").read_text())
            self.assertEqual(inputs["status"], "baseline_rejected")
            self.assertIsNone(inputs["baseline"])
            self.assertFalse((candidate / "runs").exists())

    def test_wrong_role_and_unprojected_seed_refuse_before_live_dispatch(self):
        from test_opi_eval_operator_fixture import Fixture
        from operator_flow import prepare, run_prepared, compare
        from operator_config import dumps_toml
        from operator_process import OperatorError
        fixture = Fixture(self.root, self.profile())
        with fixture.patches():
            baseline = prepare(fixture.config, "tb21", "baseline", repo=fixture.repo, index=fixture.index)
            run_prepared(baseline, "baseline", repo=fixture.repo, index=fixture.index)
            with self.assertRaises(OperatorError):
                compare(baseline / "snapshot.json", None, self.root / "wrong-role", index=fixture.index)
            self.assertFalse((self.root / "wrong-role").exists())
            fixture.profile["benchmarks"]["tb21"]["seeds"] = {"one":17}
            fixture.config.write_text(dumps_toml(fixture.profile))
            with self.assertRaises(OperatorError):
                prepare(fixture.config, "tb21", "candidate", repo=fixture.repo, index=fixture.index)
            self.assertEqual(len(list((fixture.workspace / "evaluations").glob("*/*/*/record.json"))), 1)

    def test_seed_copy_keeps_runner_subpackages_and_drops_cache_views(self):
        from operator_flow import publish_cache_seed
        cache = self.root / "family" / "cache"
        (cache / "runners/harbor/packages/harbor-atif2otel/src").mkdir(parents=True)
        (cache / "runners/harbor/packages/harbor-atif2otel/src/keep.py").write_text("runner source\n")
        (cache / "runners/harbor/.venv/lib").mkdir(parents=True)
        (cache / "runners/harbor/.venv/lib/site.py").write_text("virtualenv\n")
        (cache / "corpora/terminal-bench-2.1/tasks").mkdir(parents=True)
        (cache / "corpora/terminal-bench-2.1/tasks/task.toml").write_text("[task]\n")
        (cache / "packages").mkdir()
        (cache / "packages/view.toml").write_text("cache view\n")
        (cache / "profiles").mkdir()
        (cache / "home").mkdir()
        (cache / "inventory.json").write_text("{}\n")
        profile = {"storage": {"cache": str(self.root / "shared-cache")}}
        publish_cache_seed(profile, self.root / "family", REPO)
        seed = next((self.root / "shared-cache" / "seeds").iterdir())
        self.assertTrue((seed / "runners/harbor/packages/harbor-atif2otel/src/keep.py").is_file())
        self.assertTrue((seed / "corpora/terminal-bench-2.1/tasks/task.toml").is_file())
        self.assertFalse((seed / "runners/harbor/.venv").exists())
        for view in ("packages", "profiles", "home", "inventory.json"):
            self.assertFalse((seed / view).exists(), view)


if __name__ == "__main__":
    unittest.main()
