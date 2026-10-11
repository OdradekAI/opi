#!/usr/bin/env python3
"""Offline archive and explicit maintenance contracts on disposable evidence."""
from contextlib import contextmanager
import copy
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from datetime import datetime, timedelta, timezone

REPO = Path(__file__).resolve().parents[4]
OPERATOR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(OPERATOR))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")


class FixtureStore:
    """Minimal catalogue protocol backed by real isolated records and leases."""
    def __init__(self, workspace):
        self.workspace = workspace.resolve()
        self.evaluations = self.workspace / "evaluations"
        self.events = self.workspace / "events"
        self.leases = self.workspace / "leases"
        self.rows = {}

    @staticmethod
    def profile():
        return {"schema": "opi-eval-operator/2", "model": {"provider": "local", "model": "synthetic",
                "api": "openai-completions", "controls": {"context_window": 8192, "max_output_tokens": 4096,
                "temperature": "omitted", "top_p": "omitted"}, "thinking": {"level": "high", "required": True}},
                "baseline": {"pi_version": "1.0.0"}, "benchmarks": {"tb21": {"scope": "subset",
                "task_ids": ["one"], "repetitions": 1, "seeds": {}}}}

    def add(self, identity, *, product="opi", completed="2020-01-01T00:00:00+00:00",
            profile=None, version="1.0.0", **fields):
        root = self.evaluations / "2020/01" / identity
        family = root / "families/tb21"
        family.mkdir(parents=True)
        profile = copy.deepcopy(self.profile() if profile is None else profile)
        record = {"schema": "opi-eval-record/1", "id": identity,
              "root": str(root), "action": "baseline" if product == "pi" else "candidate",
              "created_at": fields.get("created_at", completed), "families": {"tb21": str(family)}, "profile": profile}
        write(root / "record.json", record)
        snapshot = {"schema": "opi-eval-baseline-snapshot/1", "id": identity,
                    "product": product, "version": version, "bench": "tb21",
                    "completed_at": completed, "complete": True, "status": "complete",
                    "companion": {"sha256": "a" * 64},
                    "semantic": {"common": {"admitted": True, "controls": {"reasoning": "high"}},
                                 "tasks": {"one": {"admitted": True, "package": "x"}}},
                    "attempts": [], "slots": []}
        write(family / "snapshot.json", snapshot)
        (family / "native").mkdir()
        (family / "native/trial-result.bin").write_bytes(b"sealed-native\x00\xff\n")
        row = {"id": identity, "root": str(root), "ownership": "managed", "action": "baseline" if product == "pi" else "candidate",
               "created_at": completed, "completed_at": completed, "status": "complete",
               "storage_state": "active", "selection_state": "eligible", "pinned": False,
               "references": [], "reference_unknown": [], "families": {"tb21": {
                   "root": str(family), "status": "complete", "product": product, "version": version,
                   "completed_at": completed, "complete": True, "semantic": snapshot["semantic"],
                   "companion": snapshot["companion"],
                   "snapshot": {"path": str(family / "snapshot.json"), "sha256": sha(family / "snapshot.json")}}}, "record": record}
        row.update(fields)
        self.rows[identity] = row
        return row

    def list(self, **filters):
        rows = list(self.rows.values())
        offset = filters.get("offset", 0)
        return rows[offset:offset + filters.get("limit", len(rows))]

    def show(self, identity):
        return self.rows[identity]

    def refresh(self, root):
        return next(row for row in self.rows.values() if row["root"] == str(root))

    def event(self, identity, action, **fields):
        row = self.rows[identity]
        write(self.events / identity / (action + ".json"), {"action": action, "fields": fields})
        row["lifecycle"] = fields
        if action == "archive":
            row["storage_state"] = "archived"
        elif action == "restore":
            row["storage_state"] = "active"
        elif action == "purge":
            row["storage_state"] = "purged"
        return fields

    @contextmanager
    def lease(self, identity):
        self.leases.mkdir(parents=True, exist_ok=True)
        lock = self.leases / (identity + ".json")
        descriptor = os.open(lock, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        os.close(descriptor)
        try:
            yield
        finally:
            lock.unlink()


class LifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.store = FixtureStore(self.root / "workspace")
        self.assertIsNotNone(importlib.util.find_spec("operator_lifecycle"), "lifecycle implementation is missing")
        import operator_lifecycle
        self.life = operator_lifecycle
        self.policy = {"keep_latest_per_group": 1}

    def plan(self, action="archive", ids=None):
        return self.life.maintenance_plan(self.store, self.policy, action, ids)

    def save(self, plan):
        path = self.root / "reviewed-plan.json"
        write(path, plan)
        return path

    def selected(self, plan):
        return [entry["id"] for entry in plan["selected"]]

    def test_preview_is_deterministic_and_does_not_mutate_evidence(self):
        row = self.store.add("old")
        before = {str(p): p.read_bytes() for p in Path(row["root"]).rglob("*") if p.is_file()}
        first, second = self.plan(), self.plan()
        self.assertEqual(first, second)
        self.assertEqual(self.selected(first), ["old"])
        self.assertEqual(before, {str(p): p.read_bytes() for p in Path(row["root"]).rglob("*") if p.is_file()})
        self.assertFalse((self.store.workspace / "archives").exists())

    def test_protects_baselines_pins_references_unknowns_and_leases(self):
        pi = self.store.add("pi", product="pi")
        self.store.add("pinned", pinned=True)
        target = self.store.add("target")
        self.store.add("referrer", references=[str(Path(target["root"]) / "families/tb21/snapshot.json")])
        self.store.add("unfinished", status="prepared")
        self.store.add("unknown", reference_unknown=["frozen comparison unreadable"])
        self.store.leases.mkdir(parents=True)
        (self.store.leases / "pi.json").write_text("unknown lease")
        plan = self.plan()
        self.assertEqual(self.selected(plan), [])
        self.assertTrue(all(entry["protected"] for entry in plan["inventory"]))
        self.assertIn("keep_latest_per_group", next(entry for entry in plan["inventory"] if entry["id"] == "pi")["protected"])
        self.assertEqual(pi["storage_state"], "active")

    def test_newest_count_uses_controls_without_age_exclusion(self):
        self.store.add("old", completed="2020-01-01T00:00:00+00:00")
        self.store.add("new", completed="2021-01-01T00:00:00+00:00")
        self.store.add("recent", completed="2099-01-01T00:00:00+00:00")
        self.policy["keep_latest_per_group"] = 1
        self.assertEqual(self.selected(self.plan("purge")), ["new", "old"])
        self.policy["keep_latest_per_group"] = 2
        self.assertEqual(self.selected(self.plan("purge")), ["old"])

    def test_superseded_unreferenced_pi_excess_can_archive_or_purge(self):
        self.store.add("pi-old", product="pi")
        self.store.add("pi-new", product="pi", completed="2021-01-01T00:00:00+00:00")
        self.assertEqual(self.selected(self.plan()), ["pi-old"])
        self.assertEqual(self.selected(self.plan("purge")), ["pi-old"])

    def test_apply_refuses_drift_and_modified_reviewed_plan(self):
        row = self.store.add("old")
        plan = self.plan()
        path = self.save(plan)
        (Path(row["root"]) / "families/tb21/native/trial-result.bin").write_bytes(b"drift")
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, path)
        self.assertTrue(Path(row["root"]).is_dir())
        plan = self.plan()
        plan["selected"] = []
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, self.save(plan))

    def test_apply_rechecks_added_references_and_exclusive_leases(self):
        row = self.store.add("old")
        path = self.save(self.plan())
        self.store.add("new-ref", references=[str(Path(row["root"]) / "families/tb21/snapshot.json")])
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, path)
        self.store.rows.pop("new-ref")
        with self.store.lease("old"), self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, path)

    def test_archive_frees_hot_bytes_and_restore_preserves_all_native_bytes(self):
        row = self.store.add("old")
        root = Path(row["root"])
        before = {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()}
        receipt = self.life.apply_plan(self.store, self.save(self.plan()))
        self.assertEqual(receipt["action"], "archive")
        self.assertEqual(sorted(p.name for p in root.iterdir()), ["record.json"])
        self.assertTrue(Path(row["lifecycle"]["archive_path"]).is_file())
        self.life.restore(self.store, "old")
        self.assertEqual(before, {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob("*") if p.is_file()})
        self.assertEqual(row["storage_state"], "active")

    def test_purge_keeps_record_and_tombstone_and_rejects_obsolete_policy_fields(self):
        row = self.store.add("old")
        self.store.add("new", completed="2021-01-01T00:00:00+00:00")
        plan = self.plan("purge")
        self.assertTrue((Path(row["root"]) / "families").exists())
        self.life.apply_plan(self.store, self.save(plan))
        self.assertEqual(list(Path(row["root"]).iterdir()), [Path(row["root"]) / "record.json"])
        self.assertTrue(row["lifecycle"]["tombstone"])
        self.policy["candidate_days"] = 90
        with self.assertRaises(self.life.LifecycleError):
            self.plan()

    def test_symlinks_external_dependencies_and_unknown_cleanup_are_protected(self):
        row = self.store.add("dependency")
        write(Path(row["root"]) / "families/tb21/material.json", {"runner": {"path": str(self.root / "cache/runner"), "sha256": "f" * 64}})
        cleanup = self.store.add("cleanup", status="stopped")
        write(Path(cleanup["root"]) / "cleanup-state.json", {"cleanup_state": "unknown"})
        link = self.store.add("linked")
        try:
            (Path(link["root"]) / "escape").symlink_to(self.root, target_is_directory=True)
        except OSError:
            self.store.rows.pop("linked")
        self.assertEqual(self.selected(self.plan()), [])

    def test_staged_hardlinks_block_archive_but_not_purge(self):
        # uv installs runner venv packages as hardlinks; deletion only drops
        # the tree's link, so purge selects them while archive still refuses.
        row = self.store.add("old")
        self.store.add("new", completed="2021-01-01T00:00:00+00:00")
        outside = self.root / "uv-cache/package.py"
        outside.parent.mkdir(parents=True, exist_ok=True)
        outside.write_text("shared package bytes\n")
        staged = Path(row["root"]) / "families/tb21/cache/runners/harbor/.venv/site/package.py"
        staged.parent.mkdir(parents=True, exist_ok=True)
        os.link(outside, staged)
        self.assertNotIn("old", self.selected(self.plan("archive")))
        purge = self.plan("purge")
        self.assertEqual(self.selected(purge), ["old"])
        self.life.apply_plan(self.store, self.save(purge))
        self.assertFalse(staged.exists())
        self.assertTrue(outside.exists())
        self.assertTrue(row["lifecycle"]["tombstone"])

    def test_restore_refuses_existing_destinations_corrupt_archive_and_path_attacks(self):
        row = self.store.add("old")
        self.life.apply_plan(self.store, self.save(self.plan()))
        root = Path(row["root"])
        (root / "unexpected").write_bytes(b"preserve")
        with self.assertRaises(self.life.LifecycleError):
            self.life.restore(self.store, "old")
        (root / "unexpected").unlink()
        archive = Path(row["lifecycle"]["archive_path"])
        with tarfile.open(archive, "w:gz") as stream:
            member = tarfile.TarInfo("../escaped")
            member.size = 4
            stream.addfile(member, io.BytesIO(b"evil"))
        row["lifecycle"]["archive_sha256"] = sha(archive)
        with self.assertRaises(self.life.LifecycleError):
            self.life.restore(self.store, "old")
        self.assertFalse((root.parent / "escaped").exists())
        self.assertEqual(list(root.iterdir()), [root / "record.json"])

    def test_refuses_unmanaged_root_and_plan_inside_evidence(self):
        self.store.add("imported", ownership="imported")
        row = self.store.add("old")
        self.assertEqual(self.selected(self.plan()), ["old"])
        inside = Path(row["root"]) / "plan.json"
        write(inside, self.plan())
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, inside)

    def test_drifted_nested_pin_prevents_maintenance(self):
        row = self.store.add("old")
        root = Path(row["root"])
        target = root / "families/tb21/native/trial-result.bin"
        write(root / "families/tb21/material.json", {"native": {"path": str(target), "sha256": "f" * 64}})
        self.assertEqual(self.selected(self.plan()), [])

    def test_readonly_native_bytes_round_trip(self):
        row = self.store.add("old")
        target = Path(row["root"]) / "families/tb21/native/trial-result.bin"
        target.chmod(0o444)
        try:
            self.life.apply_plan(self.store, self.save(self.plan()))
            self.life.restore(self.store, "old")
            self.assertEqual(target.read_bytes(), b"sealed-native\x00\xff\n")
        finally:
            if target.exists():
                target.chmod(0o666)

    def test_real_catalogue_archive_reindex_restore_round_trip(self):
        from operator_catalog import Store
        store = Store(self.root / "real-workspace")
        root = store.create("candidate", ["tb21"], self.store.profile())
        family = root / "families/tb21"
        family.mkdir(parents=True)
        snapshot = {"schema": "opi-eval-baseline-snapshot/1", "id": root.name, "bench": "tb21",
                    "product": "opi", "version": "1.0.0", "completed_at": "2020-01-01T00:00:00+00:00",
                    "complete": True, "status": "complete", "companion": {"sha256": "a" * 64},
                    "semantic": {"common": {"admitted": True}, "tasks": {"one": {"admitted": True}}},
                    "attempts": [], "slots": []}
        write(family / "snapshot.json", snapshot)
        write(family / "result-handoff.json", {"status": "complete", "baseline": None})
        write(family / "inputs/comparison-input.json", {"schema": "opi-eval-comparison-input/1", "baseline": None})
        store.refresh(root)
        plan = self.life.maintenance_plan(store, self.policy, "archive")
        self.assertEqual(self.selected(plan), [root.name])
        self.life.apply_plan(store, self.save(plan))
        store.reindex()
        self.assertEqual(store.show(root.name)["storage_state"], "archived")
        self.life.restore(store, root.name)
        self.assertEqual(store.show(root.name)["storage_state"], "active")
        self.assertEqual(json.loads((family / "snapshot.json").read_text()), snapshot)

    def test_archived_candidate_purge_retains_history_and_refuses_blob_drift(self):
        row = self.store.add("old")
        self.store.add("new", completed="2021-01-01T00:00:00+00:00")
        self.life.apply_plan(self.store, self.save(self.plan()))
        archive = Path(row["lifecycle"]["archive_path"])
        plan = self.plan("purge")
        self.assertEqual(self.selected(plan), ["old"])
        original = archive.read_bytes()
        archive.write_bytes(b"changed")
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, self.save(plan))
        archive.write_bytes(original)
        self.life.apply_plan(self.store, self.save(plan))
        self.assertFalse(archive.exists())
        self.assertTrue((Path(row["root"]) / "record.json").is_file())
        self.assertEqual(row["storage_state"], "purged")

    def test_apply_refreshes_current_reference_graph_after_leasing(self):
        row = self.store.add("old")
        ref = self.store.add("ref")
        plan = self.plan(ids=["old"])
        original_refresh = self.store.refresh
        def refresh(root):
            result = original_refresh(root)
            if result["id"] == ref["id"]:
                result["references"] = [str(Path(row["root"]) / "families/tb21/snapshot.json")]
            return result
        self.store.refresh = refresh
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_plan(self.store, self.save(plan))

    def test_archive_attack_stage_is_removed_without_touching_outside_bytes(self):
        row = self.store.add("old")
        self.life.apply_plan(self.store, self.save(self.plan()))
        archive = Path(row["lifecycle"]["archive_path"])
        outside = self.root / "outside"
        outside.write_bytes(b"retain")
        with tarfile.open(archive, "w:gz") as stream:
            member = tarfile.TarInfo("record.json")
            member.type = tarfile.SYMTYPE
            member.linkname = str(outside)
            stream.addfile(member)
        row["lifecycle"]["archive_sha256"] = sha(archive)
        with self.assertRaises(self.life.LifecycleError):
            self.life.restore(self.store, "old")
        self.assertEqual(outside.read_bytes(), b"retain")
        self.assertEqual(list(archive.parent.glob(".restore-*")), [])

    def test_nested_array_external_path_is_a_portability_gap(self):
        row = self.store.add("old")
        write(Path(row["root"]) / "families/tb21/material.json", {"entries": [{"root": str(self.root / "shared-cache")}]})
        self.assertEqual(self.selected(self.plan()), [])

    def test_known_retained_baseline_reference_can_archive_its_candidate(self):
        baseline = self.store.add("pi", product="pi")
        baseline_path = Path(baseline["root"]) / "families/tb21/snapshot.json"
        candidate = self.store.add("old", references=[str(baseline_path)])
        write(Path(candidate["root"]) / "families/tb21/inputs/comparison-input.json",
              {"schema": "opi-eval-comparison-input/1", "baseline": {"path": str(baseline_path), "sha256": sha(baseline_path)}})
        self.assertEqual(self.selected(self.plan()), ["old"])

    def test_reviewed_plan_errors_are_sanitized_operator_refusals(self):
        from operator_process import OperatorError
        path = self.root / "plan.json"
        path.write_text("private-secret-not-in-error")
        with self.assertRaises(OperatorError) as caught:
            self.life.apply_plan(self.store, path)
        self.assertNotIn("private-secret", str(caught.exception))

    def test_failed_history_keeps_newest_invalid_copy(self):
        for identity, days in (("recent-invalid", 150), ("old-invalid", 200)):
            completed = (datetime.now(timezone.utc) - timedelta(days=days)).isoformat()
            row = self.store.add(identity, status="incomplete", completed=completed)
            snapshot_path = Path(row["root"]) / "families/tb21/snapshot.json"
            snapshot = json.loads(snapshot_path.read_text())
            snapshot.update(status="incomplete", complete=False)
            write(snapshot_path, snapshot)
            row["families"]["tb21"]["snapshot"]["sha256"] = sha(snapshot_path)
        self.assertEqual(self.selected(self.plan("purge")), ["old-invalid"])

    def test_readonly_directory_mode_and_bytes_round_trip(self):
        row = self.store.add("old")
        native = Path(row["root"]) / "families/tb21/native"
        native.chmod(0o555)
        try:
            self.life.apply_plan(self.store, self.save(self.plan()))
            self.life.restore(self.store, "old")
            self.assertEqual((native / "trial-result.bin").read_bytes(), b"sealed-native\x00\xff\n")
            if os.name != "nt":
                self.assertEqual(native.stat().st_mode & 0o777, 0o555)
        finally:
            if native.exists():
                native.chmod(0o755)

    def test_malformed_identity_is_protected_and_invalid_ids_are_typed_refusals(self):
        row = self.store.add("old")
        snapshot_path = Path(row["root"]) / "families/tb21/snapshot.json"
        snapshot = json.loads(snapshot_path.read_text())
        snapshot["companion"] = None
        write(snapshot_path, snapshot)
        row["families"]["tb21"]["snapshot"]["sha256"] = sha(snapshot_path)
        self.assertEqual(self.selected(self.plan()), [])
        with self.assertRaises(self.life.LifecycleError):
            self.plan(ids=[{"unknown": "private-secret"}])

    def test_restore_refuses_absolute_windows_hardlink_and_duplicate_members(self):
        for attack in ("absolute", "windows", "hardlink", "duplicate"):
            with self.subTest(attack=attack):
                row = self.store.add(attack)
                self.life.apply_plan(self.store, self.save(self.plan(ids=[attack])))
                archive = Path(row["lifecycle"]["archive_path"])
                with tarfile.open(archive, "w:gz") as stream:
                    member = tarfile.TarInfo({"absolute": "/escaped", "windows": "C:/escaped"}.get(attack, "record.json"))
                    if attack == "hardlink":
                        member.type = tarfile.LNKTYPE
                        member.linkname = "../../escaped"
                    else:
                        member.size = 4
                    stream.addfile(member, None if attack == "hardlink" else io.BytesIO(b"evil"))
                    if attack == "duplicate":
                        stream.addfile(member, io.BytesIO(b"evil"))
                row["lifecycle"].update(archive_sha256=sha(archive), archive_bytes=archive.stat().st_size)
                with self.assertRaises(self.life.LifecycleError):
                    self.life.restore(self.store, attack)
                self.assertEqual(list(archive.parent.glob(".restore-*")), [])

    def test_measured_reclaimable_bytes_exclude_retained_record(self):
        row = self.store.add("old")
        entry = self.plan()["selected"][0]
        self.assertEqual(entry["reclaimable_bytes"], entry["bytes"] - (Path(row["root"]) / "record.json").stat().st_size)

    def test_operator_generated_candidate_can_archive_and_restore_offline(self):
        from operator_catalog import Store
        from operator_flow import prepare, run_prepared
        from test_opi_eval_operator import OperatorTests
        from test_opi_eval_operator_fixture import Fixture
        case = OperatorTests()
        case.root = self.root
        profile = case.profile()
        profile["schema"] = "opi-eval-operator/2"
        profile["baseline"]["max_age_days"] = 0
        profile["retention"] = self.policy
        fixture = Fixture(self.root, profile)
        lock_path = fixture.repo / "crates/opi-eval/external-locks/static/linux-x86_64.json"
        lock = json.loads(lock_path.read_text())
        lock["schema"] = "opi-eval-external-lock/static/1"
        write(lock_path, lock)
        # This producer remains inert; give its material pins real retained bytes.
        for name in ("one", "two", "uv"):
            (fixture.cache / name).write_bytes(name.encode())
        for name in ("runner", "corpora"):
            (fixture.cache / name).mkdir(exist_ok=True)
        source = fixture.fake.read_text()
        source = source.replace("'sha256':'2'*64", "'sha256':sha(cache/task)")
        source = source.replace("'sha256':'4'*64", "'sha256':sha(cache/'uv')")
        fixture.fake.write_text(source)
        with fixture.patches():
            operation = prepare(fixture.config, "tb21", "candidate", repo=fixture.repo)
            self.assertEqual(run_prepared(operation, "candidate", repo=fixture.repo)["status"], "complete")
        evaluation = operation.parent.parent
        before = {p.relative_to(evaluation).as_posix(): p.read_bytes() for p in evaluation.rglob("*") if p.is_file()}
        store = Store(fixture.workspace)
        plan = self.life.maintenance_plan(store, profile["retention"], "archive")
        self.assertEqual(self.selected(plan), [evaluation.name], [entry["protected"] for entry in plan["inventory"]])
        self.life.apply_plan(store, self.save(plan))
        self.life.restore(store, evaluation.name)
        self.assertEqual(before, {p.relative_to(evaluation).as_posix(): p.read_bytes() for p in evaluation.rglob("*") if p.is_file()})

    def test_restored_evidence_can_reuse_its_verified_archive(self):
        row = self.store.add("old")
        self.life.apply_plan(self.store, self.save(self.plan()))
        archive = Path(row["lifecycle"]["archive_path"])
        original_archive = archive.read_bytes()
        self.life.restore(self.store, "old")
        self.life.apply_plan(self.store, self.save(self.plan()))
        self.assertEqual(archive.read_bytes(), original_archive)
        self.life.restore(self.store, "old")

    def test_cli_preview_and_apply_refuse_tampering_and_protected_output(self):
        from operator_catalog import Store
        from operator_config import dumps_toml
        from test_opi_eval_operator import OperatorTests
        case = OperatorTests()
        case.root = self.root
        profile = case.profile()
        profile.update(schema="opi-eval-operator/2", retention=self.policy)
        profile["baseline"]["max_age_days"] = 0
        store = Store(Path(profile["storage"]["workspace"]))
        root = store.create("candidate", ["tb21"], profile)
        family = root / "families/tb21"
        fixture_row = FixtureStore(self.root / "template-workspace").add("fixture")
        snapshot = json.loads((Path(fixture_row["root"]) / "families/tb21/snapshot.json").read_text())
        snapshot["id"] = root.name
        write(family / "snapshot.json", snapshot)
        write(family / "inputs/comparison-input.json", {"schema": "opi-eval-comparison-input/1", "baseline": None})
        store.refresh(root)
        config, plan = self.root / "private.toml", self.root / "preview.json"
        config.write_text(dumps_toml(profile))
        def cli(*args):
            return subprocess.run([sys.executable, str(OPERATOR / "evaluate.py"), *args,
                                   "--config", str(config)], capture_output=True, text=True)
        result = cli("archive", "--id", root.name, "--plan", str(plan))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((family / "snapshot.json").is_file())
        (family / "new-evidence").write_bytes(b"drift")
        result = cli("apply-plan", "--plan", str(plan))
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("Traceback", result.stderr)
        self.assertTrue((family / "snapshot.json").is_file())
        inside = root / "review.json"
        result = cli("archive", "--id", root.name, "--plan", str(inside))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(inside.exists())

    def test_missing_reference_facts_and_unadmitted_identity_are_protected(self):
        row = self.store.add("old")
        row.pop("references")
        self.assertEqual(self.selected(self.plan()), [])
        row["references"] = []
        snapshot_path = Path(row["root"]) / "families/tb21/snapshot.json"
        snapshot = json.loads(snapshot_path.read_text())
        snapshot["semantic"]["common"]["admitted"] = False
        write(snapshot_path, snapshot)
        row["families"]["tb21"]["snapshot"]["sha256"] = sha(snapshot_path)
        self.assertEqual(self.selected(self.plan()), [])

    @unittest.skipIf(os.name == "nt", "POSIX unreadable directory contract")
    def test_unreadable_subtree_cannot_be_silently_omitted_from_inventory(self):
        row = self.store.add("old")
        hidden = Path(row["root"]) / "families/tb21/unreadable"
        hidden.mkdir()
        (hidden / "sealed.bin").write_bytes(b"retain hidden evidence")
        hidden.chmod(0)
        try:
            self.assertEqual(self.selected(self.plan()), [])
        finally:
            hidden.chmod(0o755)

    def test_provenance_allowlist_does_not_cover_material_named_handoff(self):
        row = self.store.add("old")
        write(Path(row["root"]) / "families/tb21/material/handoff.json",
              {"schema": "opi-eval-handoff/1", "execute_command": [str(self.root / "unknown-material")]})
        self.assertEqual(self.selected(self.plan()), [])


class RollingRetentionTests(unittest.TestCase):
    def setUp(self):
        LifecycleTests.setUp(self)
        self.policy = {"keep_latest_per_group": 3}

    def prune(self, **kwargs):
        self.assertTrue(hasattr(self.life, "apply_retention"), "rolling retention is missing")
        return self.life.apply_retention(self.store, self.policy, **kwargs)

    def four(self, **fields):
        return [self.store.add(f"run-{number}", completed=f"2020-01-0{number + 1}T00:00:00+00:00", **fields)
                for number in range(4)]

    def test_fourth_ended_copy_purges_oldest_without_age_or_score(self):
        rows = self.four()
        result = self.prune()
        self.assertEqual([entry["id"] for entry in result["purged"]], [rows[0]["id"]])
        self.assertEqual([row["storage_state"] for row in rows], ["purged", "active", "active", "active"])
        self.assertEqual(result["remaining_excess"], [])

    def test_opi_version_companion_and_source_do_not_reset_count(self):
        rows = self.four()
        for number, row in enumerate(rows):
            path = Path(row["root"]) / "families/tb21/snapshot.json"
            snapshot = json.loads(path.read_text())
            snapshot["version"] = f"0.{number}.0"
            snapshot["companion"] = {"sha256": str(number + 1) * 64}
            snapshot["semantic"]["common"]["source_commit"] = str(number)
            write(path, snapshot)
            row["families"]["tb21"]["snapshot"]["sha256"] = sha(path)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], ["run-0"])

    def test_pi_release_model_and_task_scope_are_distinct_buckets(self):
        rows = self.four(product="pi")
        self.store.add("pi-two", product="pi", version="2.0.0")
        model = self.store.profile()
        model["model"]["model"] = "other"
        self.store.add("other-model", profile=model)
        scope = self.store.profile()
        scope["benchmarks"]["tb21"]["repetitions"] = 2
        self.store.add("other-scope", profile=scope)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])

    def test_recent_failed_pi_copies_do_not_keep_old_unreferenced_complete_forever(self):
        rows = self.four(product="pi")
        for row in rows[1:]:
            row["status"] = "incomplete"
            path = Path(row["root"]) / "families/tb21/snapshot.json"
            snapshot = json.loads(path.read_text())
            snapshot.update(status="incomplete", complete=False)
            write(path, snapshot)
            row["families"]["tb21"]["snapshot"]["sha256"] = sha(path)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])

    def test_protected_excess_reports_pin_reference_active_and_unknown_gaps(self):
        rows = self.four()
        rows[0]["pinned"] = True
        result = self.prune()
        self.assertEqual(result["purged"], [])
        self.assertEqual([entry["id"] for entry in result["remaining_excess"]], [rows[0]["id"]])
        self.assertIn("pinned", result["remaining_excess"][0]["protected"])
        rows[0]["pinned"] = False
        rows[0]["reference_unknown"] = ["cleanup unresolved"]
        self.assertEqual(self.prune()["purged"], [])

    def test_purging_old_sourceholder_releases_old_baseline_reference(self):
        baselines = [self.store.add(f"pi-{number}", product="pi", completed=f"2020-01-0{number + 1}T00:00:00+00:00")
                     for number in range(4)]
        candidates = self.four()
        candidates[0]["references"] = [str(Path(baselines[0]["root"]) / "families/tb21/snapshot.json")]
        result = self.prune()
        self.assertEqual({entry["id"] for entry in result["purged"]}, {"run-0", "pi-0"})

    def test_retained_current_output_is_reported_when_outside_latest_three(self):
        rows = self.four()
        result = self.prune(retain_ids=[rows[0]["id"]])
        self.assertEqual(result["purged"], [])
        self.assertIn("current_output", result["remaining_excess"][0]["protected"])

    def test_finish_order_keeps_late_finishing_request_and_default_is_template_three(self):
        rows = self.four()
        rows[0]["completed_at"] = "2030-01-01T00:00:00+00:00"
        result = self.prune()
        self.assertEqual([entry["id"] for entry in result["purged"]], [rows[1]["id"]])
        self.assertEqual(self.life._policy(None), {"keep_latest_per_group": 3})
        with self.assertRaises(self.life.LifecycleError):
            self.life.apply_retention(self.store, {"keep_latest_per_group": 0})

    def test_missing_completion_uses_immutable_creation_time_and_invalid_time_protects(self):
        rows = self.four()
        rows[0].pop("completed_at")
        rows[0].pop("created_at")
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])
        invalid = self.store.add("invalid-completion", completed="2019-01-01T00:00:00+00:00")
        invalid["completed_at"] = "not-a-timestamp"
        result = self.prune()
        self.assertEqual(result["purged"], [])
        self.assertIn("completion_unknown", next(entry for entry in result["protected"]
                                                 if entry["id"] == invalid["id"])["protected"])

    def test_archived_copies_count_and_excess_archive_blob_is_purged(self):
        rows = self.four()
        plan = self.life.maintenance_plan(self.store, self.policy, "archive", [rows[0]["id"]])
        path = self.root / "archive-review.json"
        write(path, plan)
        self.life.apply_plan(self.store, path)
        archive = Path(rows[0]["lifecycle"]["archive_path"])
        self.assertTrue(archive.is_file())
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])
        self.assertFalse(archive.exists())

    def test_purge_unlinks_owned_symlink_without_following_external_target(self):
        rows = self.four()
        outside = self.root / "outside.bin"
        outside.write_bytes(b"retain caller bytes")
        alias = Path(rows[0]["root"]) / "owned-alias"
        try:
            alias.symlink_to(outside)
        except OSError:
            self.skipTest("host cannot create file symlink")
        archive_plan = self.life.maintenance_plan(self.store, self.policy, "archive", [rows[0]["id"]])
        self.assertEqual(archive_plan["selected"], [])
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])
        self.assertEqual(outside.read_bytes(), b"retain caller bytes")
        self.assertFalse(alias.is_symlink())

    def test_administrative_retention_summary_does_not_block_cleanup(self):
        rows = self.four()
        write(Path(rows[0]["root"]) / "retention-handoff.json", {"schema": "opi-eval-retention-result/1",
              "purged": [{"id": "previous", "root": str(self.root / "previous-evaluation")}], "protected": []})
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])

    def test_grouped_family_set_counts_as_one_separate_kind(self):
        standalone = self.four()
        grouped = []
        for number in range(4):
            profile = self.store.profile()
            profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
            row = self.store.add(f"group-{number}", profile=profile, completed=f"2020-01-0{number + 1}T00:00:00+00:00")
            family = Path(row["root"]) / "families/tb30"
            snapshot = json.loads((Path(row["root"]) / "families/tb21/snapshot.json").read_text())
            snapshot["bench"] = "tb30"
            write(family / "snapshot.json", snapshot)
            row["record"]["families"]["tb30"] = str(family)
            write(Path(row["root"]) / "record.json", row["record"])
            row["families"]["tb30"] = {**row["families"]["tb21"], "root": str(family),
                                       "snapshot": {"path": str(family / "snapshot.json"), "sha256": sha(family / "snapshot.json")}}
            grouped.append(row)
        result = self.prune()
        self.assertEqual({entry["id"] for entry in result["purged"]}, {standalone[0]["id"], grouped[0]["id"]})

    def grouped_preparation_failures(self):
        rows = []
        for number in range(4):
            profile = self.store.profile()
            profile["benchmarks"]["tb30"] = copy.deepcopy(profile["benchmarks"]["tb21"])
            row = self.store.add(f"failed-group-{number}", product="pi", profile=profile,
                                 completed=f"2020-01-0{number + 1}T00:00:00+00:00")
            root = Path(row["root"])
            family = root / "families/tb21"
            (family / "snapshot.json").unlink()
            (family / "native/trial-result.bin").unlink()
            (family / "native").rmdir()
            write(family / "preparation-failure.json", {"status": "preparation_failed",
                  "at": row["completed_at"], "live_dispatch": False})
            row["status"] = "preparation_failed"
            row["families"]["tb21"].update(status="preparation_failed", snapshot=None)
            unstarted = root / "families/tb30"
            row["record"]["families"]["tb30"] = str(unstarted)
            row["families"]["tb30"] = {"root": str(unstarted), "status": "not_started", "snapshot": None,
                                         "product": "pi", "version": "1.0.0"}
            write(root / "record.json", row["record"])
            write(root / "preparation-failure.json", {"schema": "opi-eval-preparation-failure/1",
                  "evaluation": str(root), "status": "preparation_failed", "live_dispatch": False,
                  "completed_at": row["completed_at"], "families": {bench: {
                      "root": family["root"], "status": family["status"], "live_dispatch": False}
                      for bench, family in row["families"].items()}})
            row["preparation_failure"] = {"path": str(root / "preparation-failure.json"),
                                           "sha256": sha(root / "preparation-failure.json")}
            rows.append(row)
        return rows

    def test_grouped_preparation_failure_prunes_owned_unstarted_wrapper(self):
        rows = self.grouped_preparation_failures()
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])

    def test_grouped_preparation_failure_requires_frozen_nonlive_binding(self):
        rows = self.grouped_preparation_failures()
        row = rows[0]
        row["preparation_failure"]["sha256"] = "f" * 64
        self.assertEqual(self.prune()["purged"], [])
        path = Path(row["preparation_failure"]["path"])
        failure = json.loads(path.read_text())
        failure["families"]["tb30"]["live_dispatch"] = None
        write(path, failure)
        row["preparation_failure"]["sha256"] = sha(path)
        self.assertEqual(self.prune()["purged"], [])
        failure["families"]["tb30"]["live_dispatch"] = False
        write(path, failure)
        row["preparation_failure"]["sha256"] = sha(path)
        write(Path(row["root"]) / "families/tb30/execution-authorization.json", {"status": "prepared"})
        self.assertEqual(self.prune()["purged"], [])
        (Path(row["root"]) / "families/tb30/execution-authorization.json").unlink()
        write(Path(row["root"]) / "families/tb30/cleanup.json", {"cleanup_state": "unconfirmed"})
        self.assertEqual(self.prune()["purged"], [])

    def test_grouped_failure_prepared_sibling_retains_frozen_reference(self):
        rows = self.grouped_preparation_failures()
        source = self.store.add("source-release", product="pi", version="2.0.0")
        source_path = Path(source["root"]) / "families/tb21/snapshot.json"
        row = rows[0]
        family = Path(row["root"]) / "families/tb30"
        write(family / "inputs/comparison-input.json", {"schema": "opi-eval-comparison-input/1",
              "baseline": {"path": str(source_path), "sha256": sha(source_path)}})
        write(family / "prepared.json", {"schema": "opi-eval-prepared-operation/1", "comparison_input": {
              "path": str(family / "inputs/comparison-input.json"), "sha256": sha(family / "inputs/comparison-input.json")}})
        row["references"] = [str(source_path)]
        row["families"]["tb30"]["status"] = "prepared"
        failure_path = Path(row["preparation_failure"]["path"])
        failure = json.loads(failure_path.read_text())
        failure["families"]["tb30"]["status"] = "prepared"
        write(failure_path, failure)
        row["preparation_failure"]["sha256"] = sha(failure_path)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [row["id"]])
        self.assertTrue(source_path.is_file())

    def comparison_rows(self, *, stopped=False):
        source = self.store.add("candidate-source")
        source_path = Path(source["root"]) / "families/tb21/snapshot.json"
        caller_outputs, rows = [], []
        for number in range(4):
            completed = f"2021-01-0{number + 1}T00:00:00+00:00"
            row = self.store.add(f"compare-{number}", completed=completed)
            root, family = Path(row["root"]), Path(row["root"]) / "families/tb21"
            (family / "snapshot.json").unlink()
            (family / "native/trial-result.bin").unlink()
            (family / "native").rmdir()
            row["action"] = row["record"]["action"] = "compare"
            row["families"]["tb21"]["snapshot"] = None
            row["references"] = [str(source_path)]
            write(root / "record.json", row["record"])
            output = self.root / "caller-results" / row["id"]
            write(output / "comparison.json", {"schema": "opi-eval-cross-run-comparison/1", "status": "baseline_missing"})
            write(output / "comparison-manifest.json", {"schema": "opi-eval-comparison-manifest/1"})
            write(family / "inputs/comparison-input.json", {"schema": "opi-eval-comparison-input/1",
                  "candidate": {"path": str(source_path), "sha256": sha(source_path)}, "baseline": None})
            result = {"schema": "opi-eval-handoff/1", "status": "complete", "operation": str(family),
                      "comparison": {"path": str(output / "comparison.json"), "sha256": sha(output / "comparison.json")},
                      "comparison_manifest": {"path": str(output / "comparison-manifest.json"), "sha256": sha(output / "comparison-manifest.json")}}
            if stopped:
                result.update(status="stopped", live_dispatch=False, cleanup="confirmed")
                row["status"] = "stopped"
            write(family / "handoff.json", result)
            write(root / "result-handoff.json", {"schema": "opi-eval-evaluation-handoff/1", "evaluation": str(root),
                  "status": "complete", "completed_at": completed, "families": {"tb21": result}})
            caller_outputs.append({p: p.read_bytes() for p in output.iterdir()})
            rows.append(row)
        return source_path, caller_outputs, rows

    def test_compare_records_roll_owned_summaries_and_keep_caller_outputs(self):
        source_path, caller_outputs, rows = self.comparison_rows()
        result = self.prune()
        self.assertEqual([entry["id"] for entry in result["purged"]], [rows[0]["id"]])
        self.assertEqual(list(Path(rows[0]["root"]).iterdir()), [Path(rows[0]["root"]) / "record.json"])
        for before in caller_outputs:
            self.assertEqual(before, {path: path.read_bytes() for path in before})
        self.assertTrue(source_path.is_file())

    def test_stopped_offline_comparison_requires_closed_nonlive_proof(self):
        source, outputs, rows = self.comparison_rows(stopped=True)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [rows[0]["id"]])
        self.assertTrue(source.is_file())
        for before in outputs:
            self.assertEqual(before, {path: path.read_bytes() for path in before})

    def test_stopped_comparison_missing_or_unknown_proof_and_live_evidence_protects(self):
        _, _, rows = self.comparison_rows(stopped=True)
        family = Path(rows[0]["root"]) / "families/tb21"
        handoff = json.loads((family / "handoff.json").read_text())
        for changes in ({"live_dispatch": None}, {"cleanup": "unknown"}):
            with self.subTest(changes=changes):
                write(family / "handoff.json", {**handoff, **changes})
                self.assertEqual(self.prune()["purged"], [])
        write(family / "handoff.json", handoff)
        write(family / "execution-authorization.json", {"schema": "opi-eval-authorization/1"})
        self.assertEqual(self.prune()["purged"], [])
        (family / "execution-authorization.json").unlink()
        write(family / "runs/attempt-1/result.json", {"status": "stopped"})
        self.assertEqual(self.prune()["purged"], [])
        (family / "runs/attempt-1/result.json").unlink()
        (family / "runs/attempt-1").rmdir()
        (family / "runs").rmdir()
        input_path = family / "inputs/comparison-input.json"
        frozen = json.loads(input_path.read_text())
        write(input_path, {**frozen, "candidate": {}})
        self.assertEqual(self.prune()["purged"], [])
        write(input_path, {**frozen, "candidate": {**frozen["candidate"], "sha256": "f" * 64}})
        self.assertEqual(self.prune()["purged"], [])
        (family / "inputs/comparison-input.json").unlink()
        self.assertEqual(self.prune()["purged"], [])

    def test_ordinary_ended_failures_prune_but_unconfirmed_cleanup_does_not(self):
        rows = self.four()
        row = rows[0]
        row["status"] = "stopped"
        path = Path(row["root"]) / "families/tb21/snapshot.json"
        snapshot = json.loads(path.read_text())
        snapshot.update(status="stopped", complete=False, stopped_reason="provider access failed")
        write(path, snapshot)
        row["families"]["tb21"]["snapshot"]["sha256"] = sha(path)
        self.assertEqual([entry["id"] for entry in self.prune()["purged"]], [row["id"]])
        other = self.store.add("uncertain", completed="2019-01-01T00:00:00+00:00")
        write(Path(other["root"]) / "effect.json", {"cleanup_state": "unconfirmed"})
        result = self.prune()
        self.assertIn("cleanup_unknown", next(entry for entry in result["remaining_excess"] if entry["id"] == "uncertain")["protected"])

    def test_retention_provenance_allowlist_is_exact(self):
        rows = self.four()
        write(Path(rows[0]["root"]) / "families/tb21/material/retention-handoff.json",
              {"schema": "opi-eval-retention-result/1", "root": str(self.root / "unknown-material")})
        result = self.prune()
        self.assertEqual(result["purged"], [])
        self.assertIn("unportable_dependency", result["remaining_excess"][0]["protected"])


if __name__ == "__main__":
    unittest.main()
