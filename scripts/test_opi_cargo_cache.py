"""Offline safety checks; every writable target is a temporary directory."""
import importlib.util
import json
import os
from pathlib import Path
import hashlib
import shutil
import subprocess
import sys
import io
import tempfile
import unittest
from unittest.mock import patch, MagicMock

SPEC = importlib.util.spec_from_file_location("cargo_cache", Path(__file__).with_name("opi-cargo-cache.py"))
cache = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(cache)


class CacheTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.target = self.base / "target"
        self.target.mkdir()
        self.env = patch.dict(os.environ, {"OPI_CARGO_CACHE_ROOT": str(self.base / "cache")})
        self.env.start()
        self.addCleanup(self.env.stop)

    def artifact(self, name):
        path = self.target / "debug" / "deps" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(name)
        return {"reason": "compiler-artifact", "package_id": "demo", "target": {"name": "demo", "kind": ["lib"]}, "profile": {"opt_level": "0"}, "features": [], "filenames": [str(path)], "fresh": True}

    def observe(self, messages, success=True, scope="scope"):
        cache.lifecycle.record(self.target, scope, "revision", messages, success, 0 if success else 1)

    def test_unmarked_status_and_lease(self):
        args = cache.parser().parse_args(["lease", "start", "--target", str(self.target), "--pid", str(os.getpid())])
        self.assertEqual(args.func(args), 0)
        self.assertFalse((self.target / cache.MARKER).exists())
        self.assertTrue(cache.lifecycle.blockers(self.target))
        args.state = "end"
        self.assertEqual(args.func(args), 0)
        self.assertFalse(cache.lifecycle.blockers(self.target))
        self.assertEqual(len(cache.entries(self.base)), 1)

    def test_only_successfully_superseded_outputs_are_candidates(self):
        old = self.artifact("libdemo-old.rlib")
        self.observe([old])
        new = self.artifact("libdemo-new.rlib")
        self.observe([new], False)
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.observe([new])
        files = cache.lifecycle.make_plan(self.target)["files"]
        self.assertEqual([x["path"] for x in files], ["debug/deps/libdemo-old.rlib"])

    def test_other_scope_protects_shared_output(self):
        old = self.artifact("libdemo-old.rlib")
        self.observe([old])
        self.observe([old], scope="other")
        self.observe([self.artifact("libdemo-new.rlib")])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])

    def test_foreign_and_legacy_leases_block(self):
        for value in ({"pid": 99999999}, {"pid": 99999999, "host": "foreign"}):
            (self.target / ".opi-cargo-lease-99999999.json").write_text(json.dumps(value))
            self.assertTrue(cache.lifecycle.blockers(self.target))

    def test_exact_apply_and_drift(self):
        self.observe([self.artifact("libdemo-old.rlib")])
        self.observe([self.artifact("libdemo-new.rlib")])
        plan = cache.lifecycle.make_plan(self.target)
        changed = self.target / plan["files"][0]["path"]
        changed.write_text("changed")
        with self.assertRaises(ValueError):
            cache.lifecycle.apply_plan(plan)
        self.assertTrue(changed.exists())

    def test_forged_receipt_cannot_delete_unknown_file(self):
        self.observe([self.artifact("libdemo-old.rlib")])
        state = self.target / cache.lifecycle.INVENTORY
        data = json.loads(state.read_text())
        data["payload"]["candidates"] = {"../outside": {}}
        state.write_text(json.dumps(data))
        with self.assertRaises(ValueError):
            cache.lifecycle.make_plan(self.target)

    def prepare_plan(self):
        self.observe([self.artifact("libdemo-old.rlib")])
        self.observe([self.artifact("libdemo-new.rlib")])
        return cache.lifecycle.make_plan(self.target)

    def test_exact_apply_retains_unknown_and_inferred_siblings(self):
        plan = self.prepare_plan()
        unknown = self.target / "debug" / "deps" / "libdemo-old.pdb"
        unknown.write_text("not positively observed")
        incremental = self.target / "debug" / "incremental" / "keep"
        incremental.parent.mkdir()
        incremental.write_text("unknown")
        self.assertEqual(cache.lifecycle.apply_plan(plan), 1)
        self.assertFalse((self.target / "debug/deps/libdemo-old.rlib").exists())
        self.assertTrue(unknown.exists())
        self.assertTrue(incremental.exists())
        self.assertTrue((self.target / "debug/deps/libdemo-new.rlib").exists())

    def test_stale_plan_rejected_after_successful_or_failed_observation(self):
        plan = self.prepare_plan()
        self.observe([], False)
        with self.assertRaisesRegex(ValueError, "inventory changed"):
            cache.lifecycle.apply_plan(plan)

    def test_live_lease_blocks_exact_apply(self):
        plan = self.prepare_plan()
        cache.lifecycle.lease(self.target, os.getpid(), True)
        with self.assertRaisesRegex(ValueError, "leases block"):
            cache.lifecycle.apply_plan(plan)

    def test_pid_reuse_and_dead_same_host_do_not_block(self):
        path = self.target / ".opi-cargo-lease-test.json"
        path.write_text(json.dumps({"schema": 2, "host": cache.lifecycle.host(), "pid": os.getpid(), "identity": "previous-process"}))
        self.assertFalse(cache.lifecycle.blockers(self.target))

    def test_abandoned_writer_lease_stays_unresolved(self):
        path = self.target / ".opi-cargo-lease-test.json"
        path.write_text(json.dumps({"schema": 2, "host": cache.lifecycle.host(), "pid": os.getpid(), "identity": "previous-process", "writer": True}))
        self.assertTrue(cache.lifecycle.blockers(self.target))

    def test_cli_writer_lease_blocks_apply_after_owner_exit(self):
        plan = self.prepare_plan()
        parent = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(30)"])
        try:
            result = self.cli("lease", "start", "--target", str(self.target), "--pid", str(parent.pid))
            self.assertEqual(result.returncode, 0, result.stderr)
        finally:
            parent.terminate()
            parent.wait()
        with patch.object(cache.lifecycle, "process_identity", return_value=None):
            self.assertTrue(cache.lifecycle.blockers(self.target))
            with self.assertRaisesRegex(ValueError, "leases block"):
                cache.lifecycle.apply_plan(plan)
        result = self.cli("lease", "end", "--target", str(self.target), "--pid", str(parent.pid))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(cache.lifecycle.apply_plan(plan), 1)

    def test_failed_run_outputs_remain_pinned_after_later_success(self):
        old = self.artifact("libdemo-old.rlib")
        self.observe([old])
        self.observe([old], False)
        self.observe([old])
        self.observe([self.artifact("libdemo-new.rlib")])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.assertIn("debug/deps/libdemo-old.rlib", cache.lifecycle.protected_paths(cache.lifecycle.load(self.target)))

    def test_incomplete_artifact_observation_preserves_previous_outputs(self):
        for error in (PermissionError("temporary sharing violation"), ValueError("unverifiable identity")):
            with self.subTest(error=type(error).__name__):
                old = self.artifact("libdemo-" + type(error).__name__ + ".rlib")
                scope = type(error).__name__
                self.observe([old], scope=scope)
                with patch.object(cache.lifecycle, "identity", side_effect=error):
                    self.observe([old], scope=scope)
                state = cache.lifecycle.load(self.target)
                self.assertTrue(state["last_run"]["observation_gap"])
                self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
                self.assertTrue(Path(old["filenames"][0]).exists())

    def test_public_binary_outside_deps_does_not_make_observation_incomplete(self):
        old = self.artifact("libdemo-old.rlib")
        self.observe([old])
        newer = self.artifact("libdemo-new.rlib")
        newer["filenames"].append(str(self.target / "debug" / "public-binary"))
        self.observe([newer])
        self.assertIsNone(cache.lifecycle.load(self.target)["last_run"]["observation_gap"])
        self.assertEqual([item["path"] for item in cache.lifecycle.make_plan(self.target)["files"]],
                         ["debug/deps/libdemo-old.rlib"])

    def test_unreadable_reported_output_revokes_existing_candidate(self):
        old = self.artifact("libdemo-old.rlib")
        newer = self.artifact("libdemo-new.rlib")
        self.observe([old])
        self.observe([newer])
        self.assertEqual(len(cache.lifecycle.make_plan(self.target)["files"]), 1)
        with patch.object(cache.lifecycle, "identity", side_effect=PermissionError("temporary sharing violation")):
            self.observe([old])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.observe([old])
        self.observe([newer])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.assertIn("debug/deps/libdemo-old.rlib", cache.lifecycle.protected_paths(cache.lifecycle.load(self.target)))

    def test_run_retains_partial_outputs_across_repeated_artifact_messages(self):
        old = self.artifact("libdemo-old.rlib")
        self.observe([old])
        newer = self.artifact("libdemo-new.rlib")
        partial = dict(newer, filenames=[*newer["filenames"], *old["filenames"]])
        args = cache.parser().parse_args(["run", "--target", str(self.target), "--", "build"])
        child = MagicMock()
        child.pid = 99999999
        child.stdout = [json.dumps(message).encode() + b"\n" for message in
                        (partial, newer, {"reason": "build-finished", "success": True})]
        child.poll.return_value = 0
        child.wait.return_value = 0
        original = cache.lifecycle.identity

        def inspect(path, content=True):
            if path == Path(old["filenames"][0]):
                raise PermissionError("temporary sharing violation")
            return original(path, content=content)

        with patch.object(cache, "invocation_identity", return_value=("scope", "revision")), patch.object(cache, "source_snapshot", return_value="snapshot"), patch.object(cache.subprocess, "Popen", return_value=child), patch.object(cache.lifecycle, "identity", side_effect=inspect), patch.object(cache.sys, "stdout", MagicMock(buffer=io.BytesIO())):
            self.assertEqual(cache.cmd_run(args), 0)
        state = cache.lifecycle.load(self.target)
        self.assertTrue(state["last_run"]["observation_gap"])
        self.assertIn("debug/deps/libdemo-new.rlib", cache.lifecycle.protected_paths(state))
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.assertFalse(cache.lifecycle.blockers(self.target))

    def test_source_drift_prevents_retiring_previous_success(self):
        self.observe([self.artifact("libdemo-old.rlib")])
        newer = cache.lifecycle.observe(self.target, self.artifact("libdemo-new.rlib"))
        cache.lifecycle.record_observed(self.target, "scope", "new", {newer[0]: newer[1]}, True, 0, gap="source drift")
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        self.assertTrue(cache.lifecycle.load(self.target)["last_run"]["build_success"])

    def test_macos_process_identity_fallback(self):
        with patch.object(cache.lifecycle.os, "name", "posix"), patch.object(cache.lifecycle.platform, "system", return_value="Darwin"):
            for result, expected in ((subprocess.CompletedProcess([], 0, "Tue Nov 3 12:00:00 2026\n", ""), "Tue Nov 3 12:00:00 2026"),
                                     (subprocess.CompletedProcess([], 1, "", ""), None),
                                     (subprocess.CompletedProcess([], 1, "", "permission denied"), "unknown")):
                with patch.object(cache.lifecycle.subprocess, "run", return_value=result):
                    self.assertEqual(cache.lifecycle.process_identity(123), expected)

    def test_guard_is_shared_filesystem_exclusive_and_never_steals_stale(self):
        with cache.lifecycle.guard(self.target):
            owner = self.target / ".opi-cargo-guard-v1" / "owner.json"
            token = owner.read_bytes()
            with patch.object(cache.lifecycle.time, "monotonic", side_effect=[0, 31]):
                with self.assertRaisesRegex(ValueError, "busy or stale"):
                    with cache.lifecycle.guard(self.target):
                        self.fail("second writer entered")
            self.assertEqual(owner.read_bytes(), token)
        self.assertFalse(owner.parent.exists())

    def test_post_cargo_save_failure_preserves_exit_and_blocks_maintenance(self):
        args = cache.parser().parse_args(["run", "--target", str(self.target), "--", "build"])
        child = MagicMock()
        child.pid = os.getpid()
        child.stdout = [b'{"reason":"build-finished","success":true}\n']
        child.poll.return_value = 0
        child.wait.return_value = 0
        stdout = MagicMock(buffer=io.BytesIO())
        with patch.object(cache, "invocation_identity", return_value=("scope", "revision")), patch.object(cache, "source_snapshot", return_value="snapshot"), patch.object(cache.subprocess, "Popen", return_value=child), patch.object(cache.lifecycle, "record_observed", side_effect=OSError("disk full")), patch.object(cache.sys, "stdout", stdout), patch.object(cache.sys, "stderr", io.StringIO()):
            self.assertEqual(cache.cmd_run(args), 0)
        self.assertTrue(cache.lifecycle.blockers(self.target))

    def test_fast_cargo_exit_and_post_build_json_are_safe(self):
        args = cache.parser().parse_args(["run", "--target", str(self.target), "--", "build"])
        artifact = self.artifact("libdemo-forged.rlib")
        child = MagicMock()
        child.pid = 99999999
        child.stdout = [b'{"reason":"build-finished","success":true}\n', json.dumps(artifact).encode() + b"\n"]
        child.poll.return_value = 0
        child.wait.return_value = 0
        stdout = MagicMock(buffer=io.BytesIO())
        with patch.object(cache, "invocation_identity", return_value=("scope", "revision")), patch.object(cache, "source_snapshot", return_value="snapshot"), patch.object(cache.subprocess, "Popen", return_value=child), patch.object(cache.sys, "stdout", stdout):
            self.assertEqual(cache.cmd_run(args), 0)
        state = cache.lifecycle.load(self.target)
        self.assertEqual(state["scopes"]["scope"], {})
        self.assertIn(b"libdemo-forged", stdout.buffer.getvalue())
        self.assertFalse(cache.lifecycle.blockers(self.target))

    def test_scope_ignores_session_env_but_tracks_effective_flags(self):
        with patch.object(cache, "workspace_root", return_value=self.base), patch.object(cache, "toolchain_identity", return_value="rustc fixture"):
            with patch.dict(os.environ, {"OPI_SESSION_TEST_VALUE": "first"}):
                first, revision = cache.invocation_identity(["build"])
            with patch.dict(os.environ, {"OPI_SESSION_TEST_VALUE": "second"}):
                second, changed_revision = cache.invocation_identity(["build"])
            self.assertEqual(first, second)
            self.assertNotEqual(revision, changed_revision)
            with patch.dict(os.environ, {"RUSTFLAGS": "--cfg cache_fixture"}):
                changed, _ = cache.invocation_identity(["build"])
            self.assertNotEqual(first, changed)

    def test_source_snapshot_is_ledger_blind(self):
        project = self.base / "source"
        (project / "src").mkdir(parents=True)
        (project / "docs/snapshots").mkdir(parents=True)
        source = project / "src/lib.rs"
        source.write_text("pub fn before() {}")
        ledger = project / ".opi-impl-state.json"
        archived = project / "docs/snapshots/assurance.rs"
        ledger.write_text("first ledger")
        archived.write_text("first assurance")
        previous = Path.cwd()
        try:
            os.chdir(project)
            first = cache.source_snapshot(self.target)
            ledger.write_text("changed ledger")
            archived.write_text("changed assurance")
            self.assertEqual(first, cache.source_snapshot(self.target))
            source.write_text("pub fn after() {}")
            self.assertNotEqual(first, cache.source_snapshot(self.target))
        finally:
            os.chdir(previous)

    def test_unobserved_artifact_keys_remain_live(self):
        first = self.artifact("libdemo-old.rlib")
        self.observe([first])
        other = self.artifact("libdemo-new.rlib")
        other["features"] = ["new-feature"]
        self.observe([other])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])

    def test_protected_paths_and_escape_never_enter_inventory(self):
        outside = self.base / "outside.rlib"
        outside.write_text("keep")
        item = self.artifact("libdemo-old.rlib")
        item["filenames"] = [str(outside), str(self.target / "debug/incremental/file")]
        self.observe([item])
        self.observe([self.artifact("libdemo-new.rlib")])
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])
        for relative in ("../outside.rlib", "/absolute", "debug/.fingerprint/file", "debug/incremental/file"):
            with self.assertRaises(ValueError):
                cache.lifecycle.output_path(self.target, relative)

    def test_symlink_ancestor_refused(self):
        link = self.base / "link"
        try:
            link.symlink_to(self.target, target_is_directory=True)
        except OSError:
            self.skipTest("symlink privilege unavailable")
        with self.assertRaisesRegex(ValueError, "linked path"):
            cache.lifecycle.safe_path(link / "new")

    def cli(self, *arguments, cwd=None):
        return subprocess.run([sys.executable, str(Path(cache.__file__).resolve()), *arguments], cwd=cwd, capture_output=True, text=True)

    def test_cli_json_status_and_unsafe_modes_fail_cleanly(self):
        result = self.cli("status", "--target", str(self.target), "--quick", "--json")
        self.assertEqual(result.returncode, 0, result.stderr)
        entry = json.loads(result.stdout)[0]
        self.assertFalse(entry["marked"])
        self.assertIsNone(entry["size"])
        for arguments in (("prune", "--root", str(self.base), "--execute"),
                          ("run", "--target", str(self.target), "--", "clean"),
                          ("run", "--target", str(self.target), "--", "build", "--target-dir=elsewhere")):
            result = self.cli(*arguments)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertNotIn("Traceback", result.stderr)

    def test_cli_plan_digest_quiescence_and_file_drift(self):
        self.prepare_plan()
        path = self.base / "plan.json"
        result = self.cli("plan", "--target", str(self.target), "--output", str(path))
        self.assertEqual(result.returncode, 0, result.stderr)
        checksum = hashlib.sha256(path.read_bytes()).hexdigest()
        self.assertEqual(result.stdout.strip(), checksum)
        for extra in (("--sha256", checksum), ("--sha256", "wrong", "--quiescent")):
            result = self.cli("apply", "--plan", str(path), *extra)
            self.assertEqual(result.returncode, 2)
        result = self.cli("apply", "--plan", str(path), "--sha256", checksum, "--quiescent")
        self.assertEqual(result.returncode, 0, result.stderr)

    @unittest.skipUnless(shutil.which("cargo"), "Cargo unavailable")
    def test_real_offline_cargo_stream_fresh_and_failed_test_exit(self):
        project = self.base / "project"
        (project / "src").mkdir(parents=True)
        (project / "Cargo.toml").write_text('[package]\nname="cache-fixture"\nversion="0.0.0"\nedition="2021"\n[workspace]\n')
        (project / "src/lib.rs").write_text('#[test] fn failing() { panic!("expected test failure"); }\n')
        first = self.cli("run", "--target", str(self.target), "--", "build", "--offline", cwd=project)
        self.assertEqual(first.returncode, 0, first.stderr)
        second = self.cli("run", "--target", str(self.target), "--", "build", "--offline", cwd=project)
        self.assertEqual(second.returncode, 0, second.stderr)
        messages = [json.loads(line) for line in second.stdout.splitlines()]
        self.assertTrue(any(m.get("reason") == "compiler-artifact" and m["fresh"] for m in messages))
        self.assertFalse(cache.lifecycle.blockers(self.target))
        failed = self.cli("run", "--target", str(self.target), "--", "test", "--offline", cwd=project)
        self.assertNotEqual(failed.returncode, 0)
        state = cache.lifecycle.load(self.target)
        self.assertTrue(state["last_run"]["build_success"])
        self.assertNotEqual(state["last_run"]["command_exit"], 0)
        self.assertEqual(cache.lifecycle.make_plan(self.target)["files"], [])


if __name__ == "__main__":
    unittest.main()
