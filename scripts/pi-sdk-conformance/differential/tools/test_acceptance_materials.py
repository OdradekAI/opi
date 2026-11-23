import hashlib
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("freeze_acceptance_materials.py")
SPEC = importlib.util.spec_from_file_location("freeze_acceptance_materials", MODULE_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class FreezeAcceptanceMaterialsTests(unittest.TestCase):
    def test_cached_catalog_is_rehashed_and_parsed(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "catalog.json"
            data = b'{"probe":[]}\n'
            path.write_bytes(data)
            revision = "sha256-" + hashlib.sha256(data).hexdigest()
            self.assertEqual(MODULE.verify_catalog(path, revision)["status"], "verified")
            path.write_bytes(b'{"changed":[]}')
            self.assertEqual(MODULE.verify_catalog(path, revision)["status"], "hash-mismatch")
            path.write_bytes(b"not json")
            revision = "sha256-" + hashlib.sha256(path.read_bytes()).hexdigest()
            self.assertEqual(MODULE.verify_catalog(path, revision)["status"], "invalid-json")

    def test_malformed_pin_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "pin.json"
            for pin in ("not-a-revision", "sha256-" + "z" * 64, 123):
                path.write_text(json.dumps({"revision": pin}), encoding="utf-8")
                with self.subTest(pin=pin), self.assertRaises(ValueError):
                    MODULE.read_revision(path)

    def test_directory_presence_is_not_content_identity(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(MODULE.tree_record(root)["status"], "empty")
            emitted = root / "index.js"
            emitted.write_bytes(b"original emitted JS")
            before = MODULE.tree_record(root)
            self.assertEqual(before["file_count"], 1)
            emitted.write_bytes(b"modified JS with unchanged source map")
            self.assertNotEqual(before["sha256"], MODULE.tree_record(root)["sha256"])
            (root / "extra.js").write_bytes(b"extra")
            self.assertEqual(MODULE.tree_record(root)["file_count"], 2)
            self.assertEqual(MODULE.tree_record(emitted)["status"], "wrong-kind")
            emitted.unlink()
            self.assertEqual(MODULE.file_record(emitted)["status"], "missing")

    def test_lock_drift_is_not_hidden_by_matching_direct_versions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            lock = root / "package-lock.json"
            original = b'{"version":"1"}'
            baseline = {"upstream": {"manifest": {"sha256": "frozen", "files": [
                {"path": "package-lock.json", "sha256": hashlib.sha256(original).hexdigest()}
            ]}}}
            lock.write_bytes(original)
            self.assertEqual(MODULE.source_comparison(root, baseline)["changes"], [])
            lock.write_bytes(b'{"version":"1", "transitive":"changed"}')
            self.assertEqual(MODULE.source_comparison(root, baseline)["changes"][0]["path"], "package-lock.json")

    def test_internal_data_validity_cannot_clear_pinned_mismatch(self):
        inspection = {
            "dependencies": [{"status": "matched-frozen-direct-record"}],
            "dist": [{"package": name, "status": "complete-source-maps-match"} for name in ("ai", "schema", "telemetry")],
            "catalog": {"status": "pinned-model-mismatch", "internal_validation": {"source": "pass", "dist": "pass"}, "source_dist_byte_mismatches": []},
        }
        self.assertEqual(MODULE.inspection_blockers(inspection), ["generated catalog differs from pinned response"])
        inspection["dependencies"][0]["status"] = "mismatch"
        self.assertIn("installed direct dependency identity", MODULE.inspection_blockers(inspection))

    def test_existing_evidence_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "existing.json"
            path.write_bytes(b"original")
            with self.assertRaises(FileExistsError):
                MODULE.write_new_json(path, {"replacement": True})
            completed = subprocess.run([
                sys.executable, "-B", str(MODULE_PATH), "--repo", directory, "--output", str(path),
            ], capture_output=True, text=True)
            self.assertEqual(completed.returncode, 2)
            self.assertIn("refusing to overwrite", completed.stderr)
            self.assertEqual(path.read_bytes(), b"original")

    def test_malformed_pin_cli_records_blocker_without_running_probe(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pin = root / MODULE.SNAPSHOT / "nix/model-catalog.json"
            pin.parent.mkdir(parents=True)
            pin.write_text('{"revision":"invalid"}', encoding="utf-8")
            output = root / "evidence.json"
            with patch.object(MODULE, "command_result", return_value={"status": "error"}) as commands:
                evidence, code = MODULE.collect_evidence(root, root / "absent-catalog", root / "absent-baseline")
            self.assertEqual(code, 2)
            self.assertEqual(evidence["catalog"]["status"], "invalid-pin")
            self.assertIn("catalog revision", evidence["blocked_gates"])
            self.assertIsNone(evidence["inspection_command"])
            self.assertEqual(commands.call_count, 5)
            MODULE.write_new_json(output, evidence)
            recorded = json.loads(output.read_text())
            self.assertEqual(recorded["schemaVersion"], MODULE.MATERIAL_SCHEMA_VERSION)
            self.assertEqual(recorded["snapshot"], str((root / MODULE.SNAPSHOT).resolve()))

    def test_legacy_or_incomplete_evidence_is_rejected_before_replay(self):
        root = Path.cwd().resolve()
        for version in (1, 2):
            with self.subTest(version=version):
                self.assertEqual(MODULE.verify_evidence(root, {"schemaVersion": version}), ["unsupported material evidence schema"])
        evidence = {
            "schemaVersion": MODULE.MATERIAL_SCHEMA_VERSION, "repository": str(root),
            "snapshot": str((root / MODULE.SNAPSHOT).resolve()),
        }
        self.assertEqual(MODULE.verify_evidence(root, evidence), ["material input inventory is incomplete"])

    def test_replay_detects_file_tree_and_catalog_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / MODULE.SNAPSHOT / "source.ts"
            source.parent.mkdir(parents=True)
            source.write_bytes(b"source")
            input_path = Path("input.rs")
            (root / input_path).write_bytes(b"driver")
            tree_path = Path("dist")
            (root / tree_path).mkdir()
            (root / tree_path / "index.js").write_bytes(b"emitted")
            baseline_path = root / "baseline.json"
            baseline = {"upstream": {"manifest": {"sha256": "frozen", "files": [
                {"path": "source.ts", "sha256": hashlib.sha256(source.read_bytes()).hexdigest()}
            ]}}}
            baseline_path.write_text(json.dumps(baseline), encoding="utf-8")
            catalog_path = root / "catalog.json"
            catalog_path.write_bytes(b'{"probe":[]}')
            revision = "sha256-" + hashlib.sha256(catalog_path.read_bytes()).hexdigest()
            evidence = {
                "schemaVersion": MODULE.MATERIAL_SCHEMA_VERSION, "repository": str(root.resolve()),
                "snapshot": str((root / MODULE.SNAPSHOT).resolve()),
                "inputs": {str(input_path): MODULE.file_record(root / input_path)},
                "trees": {str(tree_path): MODULE.tree_record(root / tree_path)},
                "catalog": MODULE.verify_catalog(catalog_path, revision),
                "baseline": {"path": str(baseline_path), **MODULE.file_record(baseline_path)},
                "source_comparison": MODULE.source_comparison(root / MODULE.SNAPSHOT, baseline),
                "inspection_command": None,
            }
            with patch.object(MODULE, "INPUT_PATHS", (input_path,)), patch.object(MODULE, "TREE_PATHS", (tree_path,)):
                self.assertEqual(MODULE.verify_evidence(root, evidence), [])
                (root / input_path).write_bytes(b"different")
                self.assertIn("input drift: input.rs", MODULE.verify_evidence(root, evidence))
                (root / input_path).write_bytes(b"driver")
                (root / tree_path / "extra.js").write_bytes(b"extra")
                self.assertIn("tree drift: dist", MODULE.verify_evidence(root, evidence))
                (root / tree_path / "extra.js").unlink()
                catalog_path.write_bytes(b'{"different":[]}')
                self.assertIn("catalog evidence drift", MODULE.verify_evidence(root, evidence))

    def test_explicit_snapshot_owns_inventory_and_inspection(self):
        with tempfile.TemporaryDirectory() as directory:
            root = (Path(directory) / "repo").resolve()
            snapshot = root.with_name("isolated snapshot")
            original = root / MODULE.SNAPSHOT
            original.mkdir(parents=True)
            lock = Path("package-lock.json")
            installed_lock = Path("node_modules/.package-lock.json")
            pin = Path("nix/model-catalog.json")
            dist = Path("packages/ai/dist")
            frozen_lock = b'{"frozen":true}'
            (original / lock).write_bytes(frozen_lock)
            catalog_path = root / "catalog.json"
            catalog_path.write_bytes(b'{"probe":[]}')
            revision = "sha256-" + hashlib.sha256(catalog_path.read_bytes()).hexdigest()
            (snapshot / pin).parent.mkdir(parents=True)
            (snapshot / pin).write_text(json.dumps({"revision": revision}), encoding="utf-8")
            (snapshot / lock).write_bytes(b'{"frozen":false}')
            (snapshot / dist).mkdir(parents=True)
            (snapshot / dist / "index.js").write_bytes(b"restored dist")
            baseline_path = root / "baseline.json"
            baseline = {"upstream": {"manifest": {"sha256": "frozen", "files": [
                {"path": lock.as_posix(), "sha256": hashlib.sha256(frozen_lock).hexdigest()}
            ]}}}
            baseline_path.write_text(json.dumps(baseline), encoding="utf-8")
            inspection = {
                "dependencies": [{"status": "matched-frozen-direct-record"}],
                "dist": [{"package": name, "status": "complete-source-maps-match"} for name in ("ai", "schema", "telemetry")],
                "catalog": {"status": "pinned-model-values-match", "internal_validation": {"source": "pass", "dist": "pass"}},
            }
            with (
                patch.object(MODULE, "INPUT_PATHS", tuple(MODULE.SNAPSHOT / path for path in (lock, installed_lock, pin))),
                patch.object(MODULE, "TREE_PATHS", (MODULE.SNAPSHOT / dist,)),
                patch.object(MODULE, "command_result", return_value={"status": "ok", "stdout": json.dumps(inspection)}) as commands,
            ):
                evidence, code = MODULE.collect_evidence(root, catalog_path, baseline_path, snapshot=snapshot)
                self.assertEqual(code, 0)
                self.assertEqual(evidence["snapshot"], str(snapshot))
                self.assertEqual(evidence["blocked_gates"], [
                    "frozen upstream source/lock provenance drift", f"input: {(snapshot / installed_lock).as_posix()}",
                ])
                self.assertEqual(evidence["inputs"][(snapshot / lock).as_posix()], MODULE.file_record(snapshot / lock))
                self.assertEqual(evidence["source_comparison"]["changes"][0]["path"], lock.as_posix())
                self.assertEqual(evidence["trees"][(snapshot / dist).as_posix()], MODULE.tree_record(snapshot / dist))
                self.assertEqual(commands.call_args_list[0].args[0][-3], str(snapshot))
                (original / lock).write_bytes(b"original tree changed after freeze")
                self.assertEqual(MODULE.verify_evidence(root, evidence), [])
                self.assertEqual(commands.call_args.args[0][-3], str(snapshot))
                self.assertEqual(
                    MODULE.verify_evidence(root, {**evidence, "snapshot": str(original)}),
                    ["material input inventory is incomplete"],
                )
                (snapshot / dist / "index.js").write_bytes(b"candidate changed after freeze")
                self.assertIn(f"tree drift: {(snapshot / dist).as_posix()}", MODULE.verify_evidence(root, evidence))


if __name__ == "__main__":
    unittest.main()
