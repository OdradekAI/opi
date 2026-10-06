#!/usr/bin/env python3
"""Offline contracts for the operator-owned, rebuildable evidence catalogue."""
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "crates/opi-eval/operator"))


class CatalogTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def profile(self):
        return {"schema": "opi-eval-operator/1",
                "model": {"provider": "local", "model": "fixture", "api_key": "do-not-retain",
                          "thinking": {"level": "high", "required": True}},
                "execution": {"trial_retries": 0}, "baseline": {"pi_version": "1.0.0"}}

    def complete(self, store, action="baseline", bench="tb21", days=0, version="1.0.0"):
        root = store.create(action, [bench], self.profile())
        return self.complete_family(store, root, action, bench, days, version)

    def complete_family(self, store, root, action="baseline", bench="tb21", days=0, version="1.0.0"):
        from operator_process import pin
        family = root / "families" / bench
        family.mkdir(parents=True)
        effective = family / "inputs/effective.json"
        effective.parent.mkdir()
        effective.write_text(json.dumps(self.profile()))
        snapshot = {"schema": "opi-eval-baseline-snapshot/1", "id": root.name, "bench": bench,
                    "product": "pi" if action == "baseline" else "opi", "version": version,
                    "completed_at": (datetime.now(timezone.utc) - timedelta(days=days)).isoformat(),
                    "complete": True, "status": "complete", "effective": pin(effective),
                    "semantic": {"common": {"admitted": True}, "tasks": {}}, "runs": []}
        (family / "snapshot.json").write_text(json.dumps(snapshot))
        (family / "result-handoff.json").write_text(json.dumps({"status": "complete", "snapshot": pin(family / "snapshot.json"), "baseline": None}))
        store.refresh(root)
        return root, family, snapshot

    def test_create_independent_dated_roots_and_sanitized_identity(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        first = store.create("candidate", ["tb21", "tb30"], self.profile())
        second = store.create("candidate", ["tb21"], self.profile())
        row = store.show(first.name)
        self.assertNotEqual(first, second)
        self.assertEqual(first.parent.parent.parent, store.workspace / "evaluations")
        self.assertEqual(row["ownership"], "managed")
        self.assertEqual(row["status"], "created")
        self.assertEqual(row["families"]["tb21"]["root"], str(first / "families/tb21"))
        self.assertNotIn("do-not-retain", (first / "record.json").read_text())
        self.assertFalse((first / "families/tb21").exists())
        self.assertEqual(len(store.list(action="candidate")), 2)

    def test_completed_pi_snapshot_retains_original_pin_and_rejects_drift(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root, family, snapshot = self.complete(store)
        original = store.snapshots(bench="tb21", pi_version="1.0.0")
        self.assertEqual(len(original), 1)
        self.assertEqual(original[0]["complete"], True)
        self.assertEqual(original[0]["self"]["path"], str(family / "snapshot.json"))
        (family / "snapshot.json").write_text(json.dumps({**snapshot, "version": "forged"}))
        row = store.show(root.name)
        self.assertEqual(row["status"], "complete")
        self.assertEqual(row["families"]["tb21"]["verification_state"], "drifted")
        self.assertEqual(store.snapshots(), [])

    def test_record_rescan_preserves_frozen_identity_and_ignores_former_database(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root, family, snapshot = self.complete(store)
        original = store.show(root.name)["families"]["tb21"]["snapshot"]
        (family / "snapshot.json").write_text(json.dumps({**snapshot, "complete": False}))
        old_database = store.workspace / "catalog.sqlite"
        old_database.write_bytes(b"former database intentionally ignored")
        scanned = Store(store.workspace)
        self.assertEqual(old_database.read_bytes(), b"former database intentionally ignored")
        summary = scanned.reindex()
        self.assertEqual(summary["schema"], "opi-eval-record-scan/1")
        self.assertEqual(summary["indexed"], 1)
        self.assertEqual(scanned.show(root.name)["families"]["tb21"]["snapshot"], original)
        self.assertEqual(scanned.show(root.name)["status"], "complete")
        self.assertEqual(scanned.snapshots(), [])
        self.assertFalse(hasattr(scanned, "catalog"))

    def test_operations_work_without_sqlite_and_create_no_query_cache(self):
        code = "import sys; sys.modules['sqlite3']=None; sys.path.insert(0, sys.argv[1]); from operator_catalog import Store; store=Store(sys.argv[2]); root=store.create('baseline',['tb21'],{}); assert store.show(root.name)['id']==root.name; assert len(store.list())==1; assert store.reindex()['indexed']==1;\nwith store.lease(root.name): pass"
        workspace = self.root / "sqlite-unavailable"
        result = subprocess.run([sys.executable, "-c", code, str(REPO / "crates/opi-eval/operator"), str(workspace)], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((workspace / "catalog.sqlite").exists())
        self.assertFalse((workspace / "catalog.json").exists())

    def test_retirement_and_storage_receipts_preserve_historical_results(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root, family, _ = self.complete(store)
        record = (root / "record.json").read_bytes()
        store.event(root.name, "pin", reason="reviewed baseline")
        store.event(root.name, "retire", reason="fresh baseline available")
        self.assertEqual(store.show(root.name)["selection_state"], "retired")
        self.assertTrue(store.show(root.name)["pinned"])
        self.assertEqual(store.snapshots(), [])
        store.event(root.name, "archive", archive_path=str(store.workspace / "archives/example.tar.gz"))
        (family / "snapshot.json").unlink()
        store.reindex()
        row = store.show(root.name)
        self.assertEqual(row["status"], "complete")
        self.assertEqual(row["storage_state"], "archived")
        self.assertEqual((root / "record.json").read_bytes(), record)
        self.assertEqual(store.snapshots(), [])

    def test_lease_excludes_other_process_and_unknown_leftovers(self):
        from operator_catalog import Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        root = store.create("candidate", ["tb21"], self.profile())
        code = "import sys; sys.path.insert(0, sys.argv[1]); from operator_catalog import Store; from operator_process import OperatorError; store=Store(sys.argv[2]);\ntry:\n with store.lease(sys.argv[3]): pass\nexcept OperatorError:\n sys.exit(7)"
        with store.lease(root.name):
            result = subprocess.run([sys.executable, "-c", code, str(REPO / "crates/opi-eval/operator"), str(store.workspace), root.name], capture_output=True, text=True)
            self.assertEqual(result.returncode, 7, result.stderr)
            self.assertTrue(store.show(root.name)["active_lease"])
        self.assertFalse(store.show(root.name)["active_lease"])
        store.leases.mkdir(exist_ok=True)
        (store.leases / (root.name + ".json")).write_text("unknown prior owner")
        with self.assertRaises(OperatorError):
            with store.lease(root.name):
                self.fail("unknown leases cannot be reclaimed automatically")

    def test_frozen_comparison_references_and_unknown_inputs_remain_visible(self):
        from operator_catalog import Store
        from operator_process import pin
        store = Store(self.root / "workspace")
        baseline, baseline_family, _ = self.complete(store)
        candidate = store.create("candidate", ["tb21"], self.profile())
        family = candidate / "families/tb21"
        family.mkdir(parents=True)
        self.assertTrue(store.refresh(candidate)["reference_unknown"])
        inputs = family / "inputs/comparison-input.json"
        inputs.parent.mkdir()
        inputs.write_text(json.dumps({"schema": "opi-eval-comparison-input/1", "baseline": pin(baseline_family / "snapshot.json")}))
        (family / "prepared.json").write_text(json.dumps({"comparison_input": pin(inputs)}))
        row = store.refresh(candidate)
        self.assertEqual(row["references"], [str(baseline_family / "snapshot.json")])
        self.assertEqual(row["reference_unknown"], [])
        inputs.write_text("forged comparison input")
        row = store.refresh(candidate)
        self.assertTrue(row["reference_unknown"])
        self.assertEqual(row["references"], [str(baseline_family / "snapshot.json")])
        self.assertTrue(store.show(baseline.name)["families"]["tb21"]["snapshot"])

    def test_filtered_queries_and_freshness_are_independent_of_historical_completion(self):
        from operator_catalog import Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        old, old_family, _ = self.complete(store, days=31)
        fresh, _, _ = self.complete(store, bench="tb30", version="1.1.0")
        candidate, _, _ = self.complete(store, action="candidate")
        self.assertEqual([row["id"] for row in store.list(bench="tb30", product="pi", version="1.1.0", model="fixture", state="complete")], [fresh.name])
        self.assertEqual(len(store.list(limit=1, offset=1)), 1)
        self.assertEqual(store.list(model="fixture' OR 1=1 --"), [])
        self.assertEqual(store.snapshots(bench="tb21", max_age_days=30), [])
        self.assertEqual(len(store.snapshots(bench="tb21", max_age_days=0)), 1)
        self.assertEqual(len(store.snapshots(pi_version="1.1.0")), 1)
        self.assertEqual(store.show(old.name)["status"], "complete")
        (old_family / "inputs/effective.json").unlink()
        # Deep native-input verification happens after deterministic latest selection.
        self.assertEqual(len(store.snapshots(bench="tb21")), 1)
        (old_family / "snapshot.json").unlink()
        self.assertEqual(store.snapshots(bench="tb21"), [])
        self.assertEqual(store.show(candidate.name)["status"], "complete")
        with self.assertRaises(OperatorError):
            store.list(limit=-1)

    def test_import_is_read_only_and_never_promotes_acceptance_evidence(self):
        from operator_catalog import Store
        from operator_process import pin
        source_store = Store(self.root / "old-workspace")
        _, family, _ = self.complete(source_store)
        index = self.root / "legacy.jsonl"
        entry = {"schema": "opi-eval-evidence-roots/1", "kind": "pi-baseline-snapshot", "path": str(family),
                 "snapshot": pin(family / "snapshot.json"), "registered": datetime.now(timezone.utc).isoformat(), "families": ["tb21"]}
        acceptance = {**entry, "kind": "local-acceptance"}
        index.write_text(json.dumps(entry) + "\n" + json.dumps(acceptance) + "\n")
        original = index.read_bytes()
        store = Store(self.root / "workspace")
        summary = store.import_index(index)
        self.assertEqual(summary["imported"], 2)
        self.assertEqual(index.read_bytes(), original)
        self.assertEqual(len(store.list(ownership="imported")), 2)
        self.assertEqual(len(store.snapshots()), 1)
        self.assertEqual(store.snapshots()[0]["self"], entry["snapshot"])
        imported_baseline = store.list(action="baseline")[0]
        self.assertEqual(store.show(imported_baseline["id"])["reference_unknown"], [])
        index.unlink()
        self.assertEqual(store.reindex()["indexed"], 2)
        self.assertEqual(len(store.snapshots()), 1)
        self.assertEqual(store.imports.is_dir(), True)

    def test_legacy_date_only_and_missing_foreign_references_remain_queryable(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        index = self.root / "legacy.jsonl"
        entry = {"schema": "opi-eval-evidence-roots/1", "kind": "acceptance",
                 "path": "/unavailable/retained/evidence", "registered": "2026-09-29",
                 "families": ["terminal-bench-2.1"], "status": "retained"}
        index.write_text(json.dumps(entry) + "\n")
        self.assertEqual(store.import_index(index)["imported"], 1)
        row = store.list(bench="tb21")[0]
        self.assertEqual(row["status"], "retained")
        self.assertEqual(row["storage_state"], "missing")
        self.assertEqual(store.show(row["id"])["root"], entry["path"])
        self.assertEqual(store.snapshots(), [])

    def test_missing_managed_root_remains_a_missing_historical_record_after_rebuild(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root, _, _ = self.complete(store)
        shutil.rmtree(root)
        row = store.show(root.name)
        self.assertEqual(row["storage_state"], "missing")
        self.assertEqual(row["status"], "complete")
        self.assertTrue(row["reference_unknown"])
        self.assertEqual(store.reindex()["indexed"], 1)
        self.assertEqual(store.show(root.name)["storage_state"], "missing")
        self.assertEqual(store.snapshots(), [])

    def test_record_controls_omit_nested_credential_fields(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        profile = self.profile()
        profile["model"]["controls"] = {"max_output_tokens": 20, "api_key": "nested-secret"}
        profile["retention"] = {"keep_latest_per_group": 3, "api_token": "nested-secret"}
        root = store.create("candidate", ["tb21"], profile)
        self.assertNotIn("nested-secret", (root / "record.json").read_text())
        self.assertEqual(store.show(root.name)["record"]["profile"]["model"]["controls"], {"max_output_tokens": 20})
        self.assertEqual(store.show(root.name)["record"]["retention"], {"keep_latest_per_group": 3})

    def test_unsupported_snapshot_contract_is_unverifiable(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root = store.create("baseline", ["tb21"], self.profile())
        family = root / "families/tb21"
        family.mkdir(parents=True)
        (family / "snapshot.json").write_text(json.dumps({"schema": "unrelated/1", "status": "complete", "product": "pi", "complete": True, "bench": "tb21"}))
        self.assertEqual(store.refresh(root)["families"]["tb21"]["verification_state"], "unverifiable")
        self.assertEqual(store.snapshots(), [])

    def test_unknown_lifecycle_receipt_fails_closed(self):
        from operator_catalog import Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        root = store.create("candidate", ["tb21"], self.profile())
        directory = store.events / root.name
        (directory / "unknown.json").write_text(json.dumps({"schema": "opi-eval-lifecycle-event/1", "id": root.name,
            "action": "silently-promote", "at": datetime.now(timezone.utc).isoformat(), "fields": {}}))
        with self.assertRaises(OperatorError):
            store.show(root.name)

    def test_final_group_handoff_preserves_family_scores_and_partial_request_status(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root = store.create("baseline", ["tb21", "tb30"], self.profile())
        _, family, _ = self.complete_family(store, root)
        pending = root / "families/tb30"
        pending.mkdir()
        (pending / "prepared.json").write_text(json.dumps({"comparison_input": None}))
        handoff = {"schema": "opi-eval-evaluation-handoff/1", "status": "incomplete", "evaluation": str(root),
                   "completed_at": "2026-09-29T18:00:00+00:00",
                   "families": {"tb21": {"status": "complete", "operation": str(family)},
                                "tb30": {"status": "not_started", "operation": str(pending)}},
                   "gap": "request-wide batch deadline exhausted"}
        (root / "result-handoff.json").write_text(json.dumps(handoff))
        row = store.refresh(root)
        self.assertEqual(row["status"], "incomplete")
        self.assertEqual(row["completed_at"], handoff["completed_at"])
        self.assertEqual(row["families"]["tb21"]["status"], "complete")
        self.assertEqual(len(store.snapshots(bench="tb21")), 1)
        store.event(root.name, "archive", archive_path=str(store.workspace / "archives/group.tar.gz"))
        (root / "result-handoff.json").unlink()
        self.assertEqual(store.show(root.name)["status"], "incomplete")

    def test_legacy_finished_request_uses_immutable_creation_time(self):
        from operator_catalog import Store
        class HistoricalDatetime(datetime):
            @classmethod
            def now(cls, tz=None):
                return datetime(2020, 1, 1, tzinfo=timezone.utc)
        store = Store(self.root / "workspace")
        with patch("operator_catalog.datetime", HistoricalDatetime):
            root = store.create("compare", ["tb21"], self.profile())
        family = root / "families/tb21"
        family.mkdir(parents=True)
        (family / "handoff.json").write_text(json.dumps({"schema": "opi-eval-handoff/1", "status": "complete"}))
        (root / "result-handoff.json").write_text(json.dumps({"schema": "opi-eval-evaluation-handoff/1",
            "status": "complete", "evaluation": str(root), "families": {"tb21": {"status": "complete"}}}))
        row = store.refresh(root)
        self.assertEqual(row["completed_at"], row["created_at"])
        self.assertEqual(row["completed_at"], "2020-01-01T00:00:00+00:00")
        store.reindex()
        self.assertEqual(store.show(root.name)["completed_at"], row["created_at"])

    def test_snapshot_order_uses_actual_utc_completion_time(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        for completed in ("2020-01-02T00:00:00+14:00", "2020-01-01T23:00:00+00:00"):
            root = store.create("baseline", ["tb21"], self.profile())
            family = root / "families/tb21"
            family.mkdir(parents=True)
            (family / "snapshot.json").write_text(json.dumps({"schema": "opi-eval-baseline-snapshot/1",
                "id": root.name, "bench": "tb21", "product": "pi", "version": "1.0.0",
                "completed_at": completed, "complete": True, "status": "complete"}))
        self.assertEqual(store.snapshots()[0]["completed_at"], "2020-01-01T23:00:00+00:00")

    def test_proven_preparation_failure_closes_missing_reference_gap(self):
        from operator_catalog import Store
        store = Store(self.root / "workspace")
        root = store.create("candidate", ["tb21"], self.profile())
        family = root / "families/tb21"
        family.mkdir(parents=True)
        failure = {"status": "preparation_failed", "live_dispatch": False,
                   "at": "2026-09-29T18:00:00+00:00", "commands": str(family / "commands")}
        (family / "preparation-failure.json").write_text(json.dumps(failure))
        (family / "handoff.json").write_text(json.dumps({"schema": "opi-eval-handoff/1",
            "status": "preparation_failed", "operation": str(family)}))
        row = store.refresh(root)
        self.assertEqual(row["status"], "preparation_failed")
        self.assertEqual(row["completed_at"], failure["at"])
        self.assertEqual(row["references"], [])
        self.assertEqual(row["reference_unknown"], [])
        self.assertTrue(row["families"]["tb21"]["pre_live"])
        (family / "execution-authorization.json").write_text("{}")
        self.assertTrue(store.refresh(root)["reference_unknown"])
        (family / "execution-authorization.json").unlink()
        (family / "preparation-failure.json").write_text(json.dumps({**failure, "live_dispatch": True}))
        drifted = store.refresh(root)
        self.assertTrue(drifted["reference_unknown"])
        self.assertEqual(drifted["completed_at"], failure["at"])

    def test_group_preparation_failure_freezes_end_and_preserves_sibling_references(self):
        from operator_catalog import Store
        from operator_process import pin
        store = Store(self.root / "workspace")
        _, baseline, _ = self.complete(store)
        root = store.create("candidate", ["tb21", "tb30", "deepswe"], self.profile())
        prepared = root / "families/tb21"
        (prepared / "inputs").mkdir(parents=True)
        comparison = prepared / "inputs/comparison-input.json"
        comparison.write_text(json.dumps({"schema": "opi-eval-comparison-input/1", "baseline": pin(baseline / "snapshot.json")}))
        (prepared / "prepared.json").write_text(json.dumps({"comparison_input": pin(comparison)}))
        failed = root / "families/tb30"
        failed.mkdir()
        (failed / "preparation-failure.json").write_text(json.dumps({"status": "preparation_failed", "live_dispatch": False,
            "at": "2026-09-29T17:00:00+00:00", "commands": str(failed / "commands")}))
        failure = {"schema": "opi-eval-preparation-failure/1", "evaluation": str(root), "status": "preparation_failed",
            "live_dispatch": False, "completed_at": "2026-09-29T18:00:00+00:00",
            "families": {bench: {"root": str(root / "families" / bench), "status": status, "live_dispatch": False}
                for bench, status in (("tb21", "prepared"), ("tb30", "preparation_failed"), ("deepswe", "not_started"))}}
        (root / "preparation-failure.json").write_text(json.dumps(failure))
        row = store.refresh(root)
        self.assertEqual(row["status"], "preparation_failed")
        self.assertEqual(row["completed_at"], failure["completed_at"])
        self.assertEqual(row["reference_unknown"], [])
        self.assertEqual(row["references"], [str(baseline / "snapshot.json")])
        self.assertEqual(row["families"]["deepswe"]["status"], "not_started")
        self.assertTrue(row["preparation_failure"])
        receipt_count = len(list((store.events / root.name).glob("*.json")))
        store.show(root.name)
        store.reindex()
        self.assertEqual(len(list((store.events / root.name).glob("*.json"))), receipt_count)
        store.event(root.name, "archive", archive_path=str(store.workspace / "archives/failure.tar.gz"))
        shutil.rmtree(root / "families")
        (root / "preparation-failure.json").unlink()
        archived = store.show(root.name)
        self.assertEqual(archived["completed_at"], failure["completed_at"])
        self.assertEqual(archived["reference_unknown"], [])
        self.assertEqual(archived["references"], row["references"])

    def test_invalid_bounded_inputs_are_typed_refusals(self):
        from operator_catalog import MAX_JSON_BYTES, Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        with self.assertRaises(OperatorError):
            store.create("candidate", [[]], self.profile())
        profile = self.profile()
        profile["model"]["controls"] = {"max_output_tokens": float("nan")}
        with self.assertRaises(OperatorError):
            store.create("candidate", ["tb21"], profile)
        profile["model"]["model"] = "x" * MAX_JSON_BYTES
        with self.assertRaises(OperatorError):
            store.create("candidate", ["tb21"], profile)
        with self.assertRaises(OperatorError):
            store.list(offset=2**80)
        with self.assertRaises(OperatorError):
            store.show("../../another-owner")

    def test_owned_symlinks_and_identity_drift_are_refused(self):
        from operator_catalog import Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        root = store.create("candidate", ["tb21"], self.profile())
        record = json.loads((root / "record.json").read_text())
        (root / "record.json").write_text(json.dumps({**record, "action": "baseline"}))
        with self.assertRaises(OperatorError):
            store.show(root.name)
        linked = self.root / "linked-workspace"
        try:
            os.symlink(store.workspace, linked, target_is_directory=True)
        except OSError:
            self.skipTest("platform cannot create a synthetic symlink")
        with self.assertRaises(OperatorError):
            Store(linked)

    def test_derivative_comparison_freezes_both_original_input_references(self):
        from operator_catalog import Store
        from operator_process import pin
        store = Store(self.root / "workspace")
        _, baseline, _ = self.complete(store)
        _, candidate, _ = self.complete(store, action="candidate")
        root = store.create("compare", ["tb21"], self.profile())
        family = root / "families/tb21"
        inputs = family / "inputs/comparison-input.json"
        inputs.parent.mkdir(parents=True)
        inputs.write_text(json.dumps({"schema": "opi-eval-comparison-input/1", "candidate": pin(candidate / "snapshot.json"), "baseline": pin(baseline / "snapshot.json")}))
        row = store.refresh(root)
        self.assertEqual(set(row["references"]), {str(candidate / "snapshot.json"), str(baseline / "snapshot.json")})
        self.assertEqual(row["reference_unknown"], [])
        inputs.write_text("corrupt frozen derivative input")
        self.assertEqual(set(store.refresh(root)["references"]), set(row["references"]))

    def test_direct_identity_operations_ignore_unrelated_damaged_records(self):
        from operator_catalog import Store
        from operator_process import OperatorError
        store = Store(self.root / "workspace")
        selected = store.create("candidate", ["tb21"], self.profile())
        damaged = store.create("candidate", ["tb30"], self.profile())
        (damaged / "record.json").write_text("unrelated corrupted immutable record")
        reopened = Store(store.workspace)
        self.assertEqual(reopened.show(selected.name)["id"], selected.name)
        self.assertEqual(reopened.list(id=selected.name)[0]["id"], selected.name)
        self.assertEqual(reopened.list(root=str(selected))[0]["id"], selected.name)
        with reopened.lease(selected.name):
            self.assertTrue(reopened.show(selected.name)["active_lease"])
        with self.assertRaises(OperatorError):
            reopened.list()

    def test_purge_graph_changes_only_after_confirmed_cleanup_and_retains_history(self):
        from operator_catalog import Store
        from operator_process import pin
        store = Store(self.root / "workspace")
        _, baseline, _ = self.complete(store)
        root, family, _ = self.complete(store, action="candidate")
        inputs = family / "inputs/comparison-input.json"
        inputs.write_text(json.dumps({"schema": "opi-eval-comparison-input/1", "baseline": pin(baseline / "snapshot.json")}))
        original_references = store.refresh(root)["references"]
        pending = store.event(root.name, "purge", cleanup_state="pending")
        self.assertEqual(pending["references"], original_references)
        self.assertIn("purge-cleanup-pending", pending["reference_unknown"])
        confirmed = store.event(root.name, "purge", cleanup_state="confirmed")
        self.assertEqual(confirmed["references"], [])
        self.assertEqual(confirmed["reference_unknown"], [])
        self.assertEqual(confirmed["status"], "complete")
        self.assertEqual(confirmed["families"]["tb21"]["references"], original_references)


if __name__ == "__main__":
    unittest.main()
