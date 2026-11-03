"""Incremental eviction tests mutate only isolated temporary targets."""
from __future__ import annotations

from contextlib import contextmanager
from concurrent.futures import ThreadPoolExecutor
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest.mock import patch

import opi_cargo_cache_eviction as eviction
REAL_METADATA = eviction.cargo_metadata


class EvictionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        (self.workspace / "Cargo.toml").write_text('[package]\nname="demo"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
        self.target = self.root / "target"
        self.target.mkdir()
        (self.target / ".opi-cargo-cache.json").write_text(json.dumps({"schema": 1, "workspace": str(self.workspace), "toolchain": "rustc fixture", "last_used": "2026-11-03T00:00:00+00:00"}))
        self.protection = self.root / "protection.json"
        self.protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [], "bindings": []}))
        self.metadata = {"workspace_root": str(self.workspace), "target_directory": str(self.target), "workspace_members": ["demo", "other"], "packages": [
            {"name": "demo", "id": "demo", "manifest_path": str(self.workspace / "Cargo.toml"), "targets": [{"name": "demo", "kind": ["lib"], "src_path": str(self.workspace / "src/lib.rs")}, {"name": "shared", "kind": ["test"], "src_path": "demo/shared.rs"}]},
            {"name": "other", "id": "other", "manifest_path": "other/Cargo.toml", "targets": [{"name": "shared", "kind": ["test"], "src_path": "other/shared.rs"}]},
        ]}
        self.metadata["resolve"] = {"nodes": [{"id": package["id"], "dependencies": [], "deps": []}
                                             for package in self.metadata["packages"]]}
        self.mock_metadata = patch.object(eviction, "cargo_metadata", return_value=self.metadata)
        self.mock_metadata.start()
        self.addCleanup(self.mock_metadata.stop)
        self.session = self.make_session("demo-123abc")

    def make_session(self, unit, name="s-abc123-abcdef-123456"):
        session = self.target / "debug/incremental" / unit / name
        session.mkdir(parents=True)
        (session.parent / (name.rsplit("-", 1)[0] + ".lock")).touch()
        version = b"1.99.1 (58cf8f9c3 2026-10-13)"
        for filename in ("dep-graph.bin", "query-cache.bin", "work-products.bin"):
            (session / filename).write_bytes(b"RSIC\0\0" + bytes([len(version)]) + version + b"payload")
        object_header = bytearray(128)
        object_header[:7] = b"\x7fELF\x02\x01\x01"
        object_header[16:24] = b"\x01\0\x3e\0\x01\0\0\0"
        object_header[52:54] = b"\x40\0"
        (session / "abc123.o").write_bytes(object_header)
        (session / "metadata.rmeta").write_bytes(b"metadata must survive")
        return session

    def plan(self, packages=None):
        return eviction.make_plan(self.target, self.workspace, packages or ["demo"], self.protection)

    def test_positive_complete_payload_eviction_preserves_metadata_and_locks(self):
        plan = self.plan()
        self.assertEqual(plan["kind"], "incremental-eviction")
        self.assertEqual(plan["summary"]["files"], 4)
        self.assertEqual(eviction.apply_plan(plan), 4)
        self.assertEqual([p.name for p in self.session.iterdir()], ["metadata.rmeta"])
        self.assertTrue(self.session.with_name("s-abc123-abcdef.lock").exists())

    def test_unknown_header_working_and_ambiguous_scope_are_skipped(self):
        (self.session / "dep-graph.bin").write_bytes(b"not a Rust cache")
        self.make_session("demo-456def", "s-abc124-abcdef-working")
        self.make_session("shared-789abc")
        plan = self.plan()
        self.assertEqual(plan["files"], [])
        self.assertTrue(plan["skipped"])

    def test_protection_skips_whole_session_and_manifest_drift_rejects(self):
        self.protection.write_text(json.dumps({"schema": 1, "files": [str(self.session / "abc123.o")], "trees": [], "bindings": []}))
        self.assertEqual(self.plan()["files"], [])
        self.protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [], "bindings": []}))
        plan = self.plan()
        self.protection.write_text(self.protection.read_text() + "\n")
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        self.assertTrue((self.session / "dep-graph.bin").exists())

    def test_whole_batch_preflight_rejects_membership_and_file_drift(self):
        plan = self.plan()
        (self.session / "new.o").write_bytes(b"unexpected")
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        self.assertTrue((self.session / "dep-graph.bin").exists())

    def test_native_lock_competition_blocks_before_delete(self):
        plan = self.plan()
        lock = self.session.with_name("s-abc123-abcdef.lock")
        script = "import sys; from pathlib import Path; import opi_cargo_cache_eviction as e;\nwith e.session_lock(Path(sys.argv[1])):\n print('locked',flush=True);sys.stdin.readline()"
        env = {**os.environ, "PYTHONPATH": str(Path(eviction.__file__).parent)}
        child = subprocess.Popen([sys.executable, "-c", script, str(lock)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env)
        try:
            self.assertEqual(child.stdout.readline().strip(), "locked")
            with self.assertRaises(ValueError):
                eviction.apply_plan(plan)
            self.assertTrue((self.session / "dep-graph.bin").exists())
        finally:
            child.communicate("release\n", timeout=10)
        self.assertEqual(child.returncode, 0)

    def test_hardlink_reclaim_estimate_requires_all_links_selected(self):
        outside = self.root / "outside.bin"
        os.link(self.session / "dep-graph.bin", outside)
        plan = self.plan()
        self.assertLess(plan["summary"]["reclaimable_bytes"], plan["summary"]["unique_allocated_bytes"])
        self.assertEqual(eviction.apply_plan(plan), 4)
        self.assertTrue(outside.exists())

    def test_selected_hardlinks_across_sessions_are_evicted_together(self):
        other = self.make_session("demo-123def", "s-abc124-abcdef-123457")
        alias = other / "dep-graph.bin"
        alias.unlink()
        os.link(self.session / "dep-graph.bin", alias)
        plan = self.plan()
        self.assertEqual(plan["summary"]["reclaimable_bytes"], plan["summary"]["unique_allocated_bytes"])
        self.assertEqual(eviction.apply_plan(plan), 8)

    def test_external_protected_alias_remains_available(self):
        evidence = self.root / "evidence.bin"
        os.link(self.session / "dep-graph.bin", evidence)
        self.protection.write_text(json.dumps({"schema": 1, "files": [str(evidence)], "trees": [], "bindings": [{"path": str(evidence), "sha256": eviction.sha256(evidence)}]}))
        plan = self.plan()
        self.assertEqual(eviction.apply_plan(plan), 4)
        self.assertEqual(eviction.sha256(evidence), plan["protection"]["bindings"][0]["sha256"])

    def test_evidence_binding_change_rejects_entire_batch(self):
        evidence = self.root / "proof.json"
        evidence.write_text("sealed proof")
        self.protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [], "bindings": [{"path": str(evidence), "sha256": eviction.sha256(evidence)}]}))
        plan = self.plan()
        evidence.write_text("changed proof")
        with self.assertRaisesRegex(ValueError, "binding changed"):
            eviction.apply_plan(plan)
        self.assertEqual(len(list(self.session.glob("*.bin"))), 3)

    def test_evidence_binding_changed_during_payload_preflight_deletes_nothing(self):
        evidence = self.root / "proof.json"
        evidence.write_text("sealed proof")
        self.protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [], "bindings": [{"path": str(evidence), "sha256": eviction.sha256(evidence)}]}))
        plan = self.plan()
        original = eviction.session_files

        def change_binding(session, protections):
            result = original(session, protections)
            evidence.write_text("new audit evidence")
            return result

        with patch.object(eviction, "session_files", side_effect=change_binding):
            with self.assertRaisesRegex(ValueError, "binding changed"):
                eviction.apply_plan(plan)
        self.assertEqual(len(list(self.session.glob("*.bin"))), 3)
        self.assertTrue((self.session / "abc123.o").exists())

    def test_metadata_changed_during_payload_preflight_deletes_nothing(self):
        plan = self.plan()
        original = eviction.session_files

        def change_metadata(session, protections):
            result = original(session, protections)
            self.metadata["packages"][0]["targets"][0]["src_path"] = "changed-source.rs"
            return result

        with patch.object(eviction, "session_files", side_effect=change_metadata):
            with self.assertRaises(ValueError):
                eviction.apply_plan(plan)
        self.assertEqual(len(list(self.session.glob("*.bin"))), 3)
        self.assertTrue((self.session / "abc123.o").exists())

    def test_new_working_session_blocks_previously_reviewed_plan(self):
        plan = self.plan()
        self.make_session("demo-123abc", "s-abc124-abcdef-working")
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        self.assertEqual(len(list(self.session.glob("*.bin"))), 3)

    def test_unresolved_lease_reported_but_blocks_apply(self):
        (self.target / ".opi-cargo-lease-legacy.json").write_text('{"pid":1}')
        plan = self.plan()
        self.assertEqual(plan["summary"]["files"], 4)
        self.assertTrue(plan["leases"])
        with self.assertRaisesRegex(ValueError, "leases block"):
            eviction.apply_plan(plan)

    def test_unowned_target_and_unknown_package_rejected(self):
        (self.target / ".opi-cargo-cache.json").unlink()
        self.metadata["target_directory"] = str(self.root / "elsewhere")
        with self.assertRaisesRegex(ValueError, "neither marked"):
            self.plan()
        with self.assertRaisesRegex(ValueError, "not a current workspace"):
            self.plan(["absent"])

    def test_windows_wsl_workspace_marker_equivalence_is_narrow(self):
        self.assertEqual(eviction.equivalent("D:\\Luiz\\Opi"), eviction.equivalent("/mnt/d/Luiz/Opi"))
        self.assertNotEqual(eviction.equivalent("/home/Luiz/Opi"), eviction.equivalent("D:/Luiz/Opi"))

    def test_shared_stem_admitted_only_when_all_owners_selected(self):
        self.make_session("shared-789abc")
        self.assertEqual(self.plan()["summary"]["files"], 4)
        self.assertEqual(self.plan(["demo", "other"])["summary"]["files"], 8)

    def collision_workspace(self, dependency):
        (self.workspace / "Cargo.toml").write_text('[workspace]\nmembers=["picked"]\nresolver="2"\n')
        picked = self.workspace / "picked"
        outsider = self.root / "outsider"
        for path, name in ((picked, "picked"), (outsider, "outsider")):
            (path / "src").mkdir(parents=True, exist_ok=True)
            (path / "src/lib.rs").write_text("pub fn answer() -> u32 { 42 }\n")
            (path / "Cargo.toml").write_text(
                f'[package]\nname="{name}"\nversion="0.1.0"\nedition="2021"\n'
                '[lib]\nname="collision"\n')
        with (picked / "Cargo.toml").open("a") as stream:
            stream.write(dependency)
        environment = {**os.environ, "CARGO_HOME": str(self.root / "cargo-home"),
                       "CARGO_TARGET_DIR": str(self.target)}
        generated = subprocess.run(
            ["cargo", "generate-lockfile", "--offline", "--manifest-path", str(self.workspace / "Cargo.toml")],
            cwd=self.workspace, env=environment, capture_output=True, text=True)
        self.assertEqual(generated.returncode, 0, generated.stderr)
        return environment

    @unittest.skipUnless(shutil.which("cargo"), "Cargo unavailable")
    def test_nonmember_dependency_target_collision_is_never_selected(self):
        self.make_session("collision-111abc")
        self.make_session("collision-222abc")
        for heading, optional in (("[dependencies]", ""),
                                  ("[dependencies]", ",optional=true"),
                                  ("[target.'cfg(target_os = \"none\")'.dependencies]", "")):
            with self.subTest(heading=heading, optional=optional):
                environment = self.collision_workspace(
                    heading + '\ndepaliasoutside={package="outsider",path="../../outsider"' + optional + '}\n')
                lock = (self.workspace / "Cargo.lock").read_bytes()
                with patch.object(eviction, "cargo_metadata", side_effect=REAL_METADATA), patch.dict(os.environ, environment):
                    plan = self.plan(["picked"])
                self.assertNotIn("collision", plan["metadata"]["admitted_stems"])
                self.assertEqual(plan["files"], [])
                self.assertEqual((self.workspace / "Cargo.lock").read_bytes(), lock)
                self.assertEqual(len(list((self.target / "debug/incremental").rglob("*.bin"))), 9)

    def test_same_package_name_does_not_select_external_package_id(self):
        self.metadata["packages"].append({
            "name": "demo", "id": "external-demo", "manifest_path": "external/Cargo.toml", "dependencies": [],
            "targets": [{"name": "demo", "kind": ["lib"], "src_path": "external/src/lib.rs"}],
        })
        self.metadata["resolve"]["nodes"].append({"id": "external-demo", "dependencies": [], "deps": []})
        self.assertEqual(self.plan()["files"], [])

    def test_missing_resolved_dependency_record_refuses_plan(self):
        self.metadata["resolve"]["nodes"][0].update(
            dependencies=["missing"],
            deps=[{"name": "outside", "pkg": "missing", "dep_kinds": [{"kind": None, "target": None}]}])
        with self.assertRaisesRegex(ValueError, "incomplete dependency ownership"):
            self.plan()

    def test_partial_or_escaping_selection_cannot_be_applied(self):
        plan = self.plan()
        plan["files"].pop()
        plan["sessions"][0]["file_count"] -= 1
        plan["summary"] = eviction.space_summary(plan["files"], 1)
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        plan = self.plan()
        plan["files"][0]["path"] = "../outside.bin"
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)

    def test_executable_masquerading_as_object_skips_whole_session(self):
        path = self.session / "abc123.o"
        contents = bytearray(path.read_bytes())
        contents[16:18] = b"\x02\0"
        path.write_bytes(contents)
        self.assertEqual(self.plan()["files"], [])

    def test_tree_protection_and_target_local_manifest_are_rejected(self):
        self.protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [str(self.session.parent)], "bindings": []}))
        self.assertEqual(self.plan()["files"], [])
        inside = self.target / "protection.json"
        inside.write_bytes(self.protection.read_bytes())
        with self.assertRaisesRegex(ValueError, "outside target"):
            eviction.make_plan(self.target, self.workspace, ["demo"], inside)

    def test_target_replacement_and_changed_payload_reject_before_unlink(self):
        plan = self.plan()
        (self.session / "abc123.o").write_bytes(b"changed")
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        self.assertTrue((self.session / "dep-graph.bin").exists())
        plan = self.plan()
        retired = self.root / "retired"
        self.target.rename(retired)
        self.target.mkdir()
        with self.assertRaisesRegex(ValueError, "replaced"):
            eviction.apply_plan(plan)
        self.assertTrue((retired / self.session.relative_to(self.target) / "dep-graph.bin").exists())

    def test_planning_leaves_target_tree_unchanged(self):
        def snapshot():
            return {p.relative_to(self.target).as_posix(): (p.stat().st_ino, p.stat().st_size, p.stat().st_mtime_ns)
                    for p in self.target.rglob("*")}
        before = snapshot()
        self.plan()
        self.assertEqual(snapshot(), before)
        self.assertFalse((self.target / ".opi-cargo-guard-v1").exists())

    @unittest.skipUnless(os.name == "nt", "Windows attribute-handle optimization")
    def test_windows_metadata_observation_never_opens_payload_for_content(self):
        path = self.session / "abc123.o"
        expected = eviction.file_info(path, header=True)
        expected.pop("header")
        with patch.object(Path, "open", side_effect=AssertionError("content permission requested")):
            self.assertEqual(eviction.file_info(path, header=False), expected)

    @unittest.skipUnless(os.name == "nt", "Windows bounded header reader")
    def test_windows_header_pool_is_bounded_reused_and_preserves_order(self):
        other = self.make_session("demo-123def", "s-abc124-abcdef-123457")
        contents = (self.session / "abc123.o").read_bytes()
        for session in (self.session, other):
            for index in range(270):
                (session / f"extra-{index:04}.o").write_bytes(contents)
        pools = []
        telemetry = {"active": 0, "max_active": 0}
        gate = threading.Lock()
        original = eviction.file_info

        class TrackingPool(ThreadPoolExecutor):
            def __init__(self, *args, **kwargs):
                super().__init__(*args, **kwargs)
                self.pending = self.peak = 0
                self.pending_gate = threading.Lock()
                pools.append(self)

            def submit(self, *args, **kwargs):
                with self.pending_gate:
                    self.pending += 1
                    self.peak = max(self.peak, self.pending)
                future = super().submit(*args, **kwargs)

                def finished(_):
                    with self.pending_gate:
                        self.pending -= 1

                future.add_done_callback(finished)
                return future

        def observe(path, header=False):
            if not header:
                return original(path, header=header)
            with gate:
                telemetry["active"] += 1
                telemetry["max_active"] = max(telemetry["max_active"], telemetry["active"])
            try:
                time.sleep(0.001)
                return original(path, header=True)
            finally:
                with gate:
                    telemetry["active"] -= 1

        with patch.object(eviction, "ThreadPoolExecutor", TrackingPool, create=True), patch.object(eviction, "file_info", side_effect=observe):
            plan = self.plan()
            self.assertEqual(len(pools), 1)
            for session in plan["sessions"]:
                paths = [item["path"] for item in plan["files"][session["file_start"]:session["file_start"] + session["file_count"]]]
                self.assertEqual(paths, sorted(paths))
            self.assertEqual(eviction.apply_plan(plan), 548)
            self.assertEqual(len(pools), 2)
        self.assertGreater(telemetry["max_active"], 1)
        self.assertLessEqual(telemetry["max_active"], 8)
        self.assertEqual(telemetry["active"], 0)
        self.assertTrue(all(pool.peak <= 256 and pool.pending == 0 for pool in pools))

    @unittest.skipUnless(os.name == "nt", "Windows bounded header reader")
    def test_failed_parallel_header_reads_finish_before_session_unlock(self):
        active = {"count": 0, "peak": 0}
        gate = threading.Lock()
        original = eviction.file_info
        original_lock = eviction.session_lock

        def observe(path, header=False):
            if not header:
                return original(path, header=False)
            with gate:
                active["count"] += 1
                active["peak"] = max(active["peak"], active["count"])
            try:
                if path.name == "abc123.o":
                    raise ValueError("expected header failure")
                time.sleep(0.03)
                return original(path, header=True)
            finally:
                with gate:
                    active["count"] -= 1

        @contextmanager
        def checked_lock(path):
            with original_lock(path):
                try:
                    yield
                finally:
                    self.assertEqual(active["count"], 0, "background read outlived session lock")

        with patch.object(eviction, "file_info", side_effect=observe), patch.object(eviction, "session_lock", checked_lock):
            plan = self.plan()
        self.assertEqual(plan["files"], [])
        self.assertGreater(active["peak"], 1)
        self.assertEqual(active["count"], 0)

    def test_forged_unit_outside_incremental_root_whitelist_rejected(self):
        plan = self.plan()
        forged = self.target / "ordinary/folder/deeper/incremental"
        forged.mkdir(parents=True)
        # This is structurally deeper than Cargo's admitted profile locations.
        self.session.parent.rename(forged / self.session.parent.name)
        unit = forged / self.session.parent.name
        session = unit / self.session.name
        plan["units"][0]["path"] = unit.relative_to(self.target).as_posix()
        plan["units"][0]["identity"] = eviction.directory_identity(unit)
        plan["units"][0]["members"] = eviction.member_digest(unit, unit=True)
        plan["incremental_roots"].append({"path": forged.relative_to(self.target).as_posix(), "identity": eviction.directory_identity(forged)})
        plan["sessions"][0]["path"] = session.relative_to(self.target).as_posix()
        plan["sessions"][0]["lock"] = (unit / "s-abc123-abcdef.lock").relative_to(self.target).as_posix()
        for item in plan["files"]:
            item["path"] = (session / Path(item["path"]).name).relative_to(self.target).as_posix()
        with self.assertRaises(ValueError):
            eviction.apply_plan(plan)
        self.assertTrue((session / "dep-graph.bin").exists())

    def test_unknown_or_nested_session_contents_are_not_partially_selected(self):
        (self.session / "important.txt").write_text("not a compiler payload")
        self.assertEqual(self.plan()["files"], [])
        (self.session / "important.txt").unlink()
        (self.session / "nested").mkdir()
        self.assertEqual(self.plan()["files"], [])

    def test_standard_coff_bigobj_header_is_recognized(self):
        data = bytearray(96)
        data[:8] = b"\0\0\xff\xff\x02\0\x64\x86"
        data[12:28] = bytes.fromhex("c7a1bad1eebaa94baf20faf66aa4dcb8")
        data[44:48] = b"\x01\0\0\0"
        self.assertTrue(eviction.payload_header("code.o", bytes(data), len(data)))
        data[12] ^= 1
        with self.assertRaises(ValueError):
            eviction.payload_header("code.o", bytes(data), len(data))

    def test_symlink_payload_is_not_selected(self):
        alias = self.session / "linked.o"
        try:
            alias.symlink_to(self.session / "abc123.o")
        except OSError:
            self.skipTest("symlink privilege unavailable")
        self.assertEqual(self.plan()["files"], [])

    @unittest.skipUnless(shutil.which("cargo"), "Cargo unavailable")
    def test_real_offline_cargo_payload_format_and_rebuild_after_eviction(self):
        project = self.root / "real-project"
        (project / "src").mkdir(parents=True)
        (project / "Cargo.toml").write_text('[package]\nname="real-demo"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
        source = project / "src/lib.rs"
        source.write_text('pub fn value() -> u32 { 41 }\n')
        target = self.root / "real-target"
        environment = {**os.environ, "CARGO_INCREMENTAL": "1", "CARGO_TARGET_DIR": str(target)}
        command = ["cargo", "build", "--offline", "--manifest-path", str(project / "Cargo.toml")]
        built = subprocess.run(command, cwd=project, env=environment, capture_output=True, text=True)
        self.assertEqual(built.returncode, 0, built.stderr)
        with patch.object(eviction, "cargo_metadata", side_effect=REAL_METADATA), patch.dict(os.environ, environment):
            plan = eviction.make_plan(target, project, ["real-demo"], self.protection)
            self.assertGreater(plan["summary"]["files"], 3, plan["skipped"])
            self.assertEqual(eviction.apply_plan(plan), plan["summary"]["files"])
        source.write_text('pub fn value() -> u32 { 42 }\n')
        rebuilt = subprocess.run(command, cwd=project, env=environment, capture_output=True, text=True)
        self.assertEqual(rebuilt.returncode, 0, rebuilt.stderr)


if __name__ == "__main__":
    unittest.main()
