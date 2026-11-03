"""Exercise eviction through the CLI; all build/deletion targets are temporary."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch


CLI = Path(__file__).with_name("opi-cargo-cache.py").resolve()


class EvictionCliTests(unittest.TestCase):
    def test_concurrently_created_output_is_not_overwritten(self):
        spec = importlib.util.spec_from_file_location("cache_eviction_cli_under_test", CLI)
        cache = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cache)
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory).resolve()
            target = base / "target"
            target.mkdir()
            protection = base / "protection.json"
            protection.write_text(json.dumps({"schema": 1, "files": [], "trees": [], "bindings": []}))
            output = base / "review.json"
            args = cache.parser().parse_args([
                "plan-eviction", "--target", str(target), "--workspace", str(base),
                "--package", "demo", "--protection", str(protection), "--output", str(output),
            ])

            def create_concurrent_file(*_):
                output.write_text("another task's evidence")
                return {"summary": {}}

            with patch("opi_cargo_cache_eviction.make_plan", side_effect=create_concurrent_file):
                with self.assertRaises(FileExistsError):
                    args.func(args)
            self.assertEqual(output.read_text(), "another task's evidence")

    def test_help_describes_explicit_eviction_inputs(self):
        result = subprocess.run(
            [sys.executable, str(CLI), "plan-eviction", "--help"],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        for option in ("--target", "--workspace", "--package", "--protection", "--output"):
            self.assertIn(option, result.stdout)

    @unittest.skipUnless(shutil.which("cargo") and shutil.which("rustc"), "Rust toolchain required")
    def test_real_offline_plan_and_digest_bound_temporary_apply(self):
        with tempfile.TemporaryDirectory(prefix="opi-cache-eviction-cli-") as directory:
            base = Path(directory).resolve()
            workspace = base / "workspace"
            workspace.mkdir()
            (workspace / "src").mkdir()
            (workspace / "src/lib.rs").write_text("pub fn answer() -> u32 { 42 }\n")
            manifest = workspace / "Cargo.toml"
            manifest.write_text(
                '[package]\nname="cache-eviction-cli-fixture"\nversion="0.1.0"\n'
                'edition="2021"\n[profile.dev]\nincremental=true\n'
            )
            target = base / "target"
            env = os.environ.copy()
            env.update(CARGO_HOME=str(base / "cargo-home"), CARGO_INCREMENTAL="1")
            env.pop("CARGO_TARGET_DIR", None)
            env["OPI_CARGO_CACHE_ROOT"] = str(base / "cache-root")
            build = subprocess.run(
                ["cargo", "build", "--offline", "--manifest-path", str(manifest),
                 "--target-dir", str(target)], cwd=workspace, env=env,
                capture_output=True, text=True,
            )
            self.assertEqual(build.returncode, 0, build.stderr)
            compiler = subprocess.check_output(["rustc", "-Vv"], env=env, text=True).strip()
            (target / ".opi-cargo-cache.json").write_text(json.dumps({
                "schema": 1, "workspace": str(workspace), "toolchain": compiler,
                "last_used": "2026-01-01T00:00:00+00:00",
            }))
            protection = base / "protection.json"
            sealed = base / "sealed-evidence"
            sealed.mkdir()
            protection.write_text(json.dumps({
                "schema": 1, "files": [], "trees": [str(sealed)], "bindings": [{
                    "path": str(manifest), "sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
                }],
            }))
            output = base / "plan.json"

            def invoke(*arguments):
                return subprocess.run(
                    [sys.executable, str(CLI), *arguments], cwd=workspace,
                    env=env, capture_output=True, text=True,
                )

            def snapshot():
                return {p.relative_to(target).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in target.rglob("*") if p.is_file()}

            before = snapshot()
            rejected_output = sealed / "new-plan.json"
            rejected = invoke("plan-eviction", "--target", str(target), "--workspace", str(workspace),
                              "--package", "cache-eviction-cli-fixture", "--protection", str(protection),
                              "--output", str(rejected_output))
            self.assertNotEqual(rejected.returncode, 0, "planner wrote inside sealed evidence")
            self.assertFalse(rejected_output.exists())
            result = invoke("plan-eviction", "--target", str(target), "--workspace", str(workspace),
                            "--package", "cache-eviction-cli-fixture", "--protection", str(protection),
                            "--output", str(output))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(before, snapshot(), "planning mutated the compiler cache")
            plan = json.loads(output.read_text())
            self.assertEqual(plan["kind"], "incremental-eviction")
            self.assertGreater(plan["summary"]["files"], 0)
            digest = hashlib.sha256(output.read_bytes()).hexdigest()
            self.assertIn(digest, result.stdout)
            duplicate = invoke("plan-eviction", "--target", str(target), "--workspace", str(workspace),
                               "--package", "cache-eviction-cli-fixture", "--protection", str(protection),
                               "--output", str(output))
            self.assertNotEqual(duplicate.returncode, 0, "existing review artifact was overwritten")
            self.assertIn("exists", duplicate.stderr)
            self.assertEqual(hashlib.sha256(output.read_bytes()).hexdigest(), digest)
            rejected = invoke("apply", "--plan", str(output), "--sha256", digest)
            self.assertNotEqual(rejected.returncode, 0)
            self.assertIn("quiescent", rejected.stderr)
            rejected = invoke("apply", "--plan", str(output), "--sha256", "0" * 64, "--quiescent")
            self.assertNotEqual(rejected.returncode, 0)
            self.assertEqual(before, snapshot())
            applied = invoke("apply", "--plan", str(output), "--sha256", digest, "--quiescent")
            self.assertEqual(applied.returncode, 0, applied.stderr)
            after = snapshot()
            removed = set(before) - set(after)
            self.assertEqual(len(removed), plan["summary"]["files"])
            self.assertTrue(all("/incremental/" in p and p.endswith((".bin", ".o")) for p in removed))
            self.assertEqual({p: h for p, h in before.items() if p not in removed}, after)
            (workspace / "src/lib.rs").write_text("pub fn answer() -> u32 { 43 }\n")
            rebuilt = subprocess.run(
                ["cargo", "build", "--offline", "--locked", "--manifest-path", str(manifest),
                 "--target-dir", str(target)], cwd=workspace, env=env,
                capture_output=True, text=True,
            )
            self.assertEqual(rebuilt.returncode, 0, rebuilt.stderr)
            self.assertTrue(list((target / "debug/incremental").rglob("dep-graph.bin")))


if __name__ == "__main__":
    unittest.main()
