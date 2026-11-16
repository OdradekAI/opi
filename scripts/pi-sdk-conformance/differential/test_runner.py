import hashlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
from compare_lifecycle import SCENARIOS
import run_lifecycle_differential as runner
from run_lifecycle_differential import verdict


class LifecycleRunnerTests(unittest.TestCase):
    def cases(self, status="difference"):
        return [{"scenario": name, "comparison": {"status": status}} for name in SCENARIOS]

    def test_failures_cannot_be_hidden_by_blocked_materials(self):
        cases = self.cases()
        cases[0]["comparison"]["status"] = "fail"
        self.assertEqual(verdict(cases, ["lock drift"]), ("FAIL", 1))

    def test_blockers_and_unapproved_differences_are_not_pass(self):
        self.assertEqual(verdict(self.cases(), ["lock drift"]), ("BLOCKED_WITH_RECORDED_OBSERVATIONS", 2))
        cases = self.cases("pass")
        self.assertEqual(verdict(cases, []), ("SHARED_OBSERVATIONS_PASS", 0))
        cases[-1]["comparison"]["status"] = "difference"
        self.assertEqual(verdict(cases, []), ("DIFFERENCES_REQUIRE_DECISION", 0))

    def test_missing_and_duplicate_cases_fail(self):
        self.assertEqual(verdict([], []), ("INCOMPLETE_CASES", 1))
        cases = self.cases()
        cases[1]["scenario"] = cases[0]["scenario"]
        self.assertEqual(verdict(cases, []), ("INCOMPLETE_CASES", 1))

    def test_cli_refuses_to_overwrite_evidence_before_running_drivers(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "existing.json"
            output.write_bytes(b"original")
            result = subprocess.run([
                sys.executable, "-B", str(Path(__file__).with_name("run_lifecycle_differential.py")),
                "--repo", directory, "--materials", str(Path(directory) / "missing.json"),
                "--output", str(output),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn("refusing to overwrite", result.stderr)
            self.assertEqual(output.read_bytes(), b"original")

    def test_cli_rejects_unbound_snapshot_before_running_drivers(self):
        freezer = runner.material_tools(runner.REPO_ROOT)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            materials, output = root / "materials.json", root / "output.json"
            materials.write_text(json.dumps({
                "schemaVersion": freezer.MATERIAL_SCHEMA_VERSION,
                "repository": str(runner.REPO_ROOT.resolve()),
                "snapshot": "relative-upstream",
            }), encoding="utf-8")
            result = subprocess.run([
                sys.executable, "-B", str(Path(runner.__file__)),
                "--repo", str(runner.REPO_ROOT), "--materials", str(materials), "--output", str(output),
            ], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2, result.stderr)
            recorded = json.loads(output.read_bytes())
            self.assertEqual(recorded["verdict"], "STALE_MATERIALS")
            self.assertIn("material evidence requires an absolute upstream snapshot path", recorded["material_drift"])
            self.assertEqual(recorded["cases"], [])
            self.assertNotIn("rust_build", recorded)

    def test_replays_bound_snapshot_and_rejects_replaced_manifest(self):
        freezer = runner.material_tools(runner.REPO_ROOT)
        for replace_manifest in (False, True):
            with self.subTest(replace_manifest=replace_manifest), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                snapshot = str(root / "isolated upstream")
                materials, output = root / "materials.json", root / "output.json"
                blockers = ["frozen upstream source/lock provenance drift"]
                manifest = {"snapshot": snapshot, "blocked_gates": blockers}
                manifest_bytes = json.dumps(manifest).encode("utf-8")
                materials.write_bytes(manifest_bytes)

                def rust_trace(*_):
                    if replace_manifest:
                        materials.write_bytes(manifest_bytes + b"\n")
                    return {"trace": {}}

                with (
                    patch.object(runner, "material_tools", return_value=freezer),
                    patch.object(freezer, "verify_evidence", return_value=[]) as verify,
                    patch.object(runner, "build_rust_driver", return_value={}),
                    patch.object(runner, "run_json", return_value={"trace": {}}) as ts,
                    patch.object(runner, "run_rust_trace", side_effect=rust_trace),
                    patch.object(runner, "compare_traces", return_value={"status": "difference"}),
                    redirect_stdout(io.StringIO()),
                ):
                    code = runner.main(["--materials", str(materials), "--output", str(output)])
                self.assertEqual(code, 2)
                self.assertEqual(verify.call_count, 2)
                self.assertEqual(ts.call_count, 3 * len(SCENARIOS))
                self.assertTrue(all(call.args[0][2:4] == ["--snapshot", snapshot] for call in ts.call_args_list))
                recorded = json.loads(output.read_bytes())
                self.assertEqual(recorded["materialsEvidence"], {
                    "path": str(materials), "sha256": hashlib.sha256(manifest_bytes).hexdigest(), "snapshot": snapshot,
                })
                self.assertEqual(recorded["blocked_gates"], blockers)
                if replace_manifest:
                    self.assertEqual(recorded["verdict"], "MATERIALS_CHANGED_DURING_RUN")
                    self.assertEqual(recorded["material_drift"], ["material manifest bytes changed"])
                else:
                    self.assertEqual(recorded["verdict"], "BLOCKED_WITH_RECORDED_OBSERVATIONS")


if __name__ == "__main__":
    unittest.main()
