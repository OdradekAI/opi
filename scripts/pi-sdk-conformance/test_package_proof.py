"""Offline safety and evidence tests for the packaged public-SDK consumer."""

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
from types import SimpleNamespace
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location(
    "package_proof", Path(__file__).with_name("run_early_package_proof.py")
)
proof = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(proof)


class PackageProofTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-package-proof-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.target = self.root / "target"
        self.target.mkdir()
        (self.target / ".opi-cargo-cache.json").write_text(
            json.dumps({"workspace": str(self.repo)}), encoding="utf-8"
        )

    def test_fresh_external_scratch_and_marked_workspace_cache_are_accepted(self):
        proof.validate_paths(self.repo, self.root / "scratch", self.target)
        self.assertFalse((self.root / "scratch").exists())

    def test_existing_scratch_is_refused_without_deletion(self):
        scratch = self.root / "scratch"
        scratch.mkdir()
        sentinel = scratch / "keep.txt"
        sentinel.write_text("keep", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "already exists"):
            proof.validate_paths(self.repo, scratch, self.target)
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_workspace_and_cache_scratch_paths_are_refused(self):
        for scratch in [self.repo / "scratch", self.target / "scratch"]:
            with self.subTest(scratch=scratch), self.assertRaisesRegex(ValueError, "outside"):
                proof.validate_paths(self.repo, scratch, self.target)

    def test_unmarked_or_other_workspace_cache_is_refused(self):
        marker = self.target / ".opi-cargo-cache.json"
        marker.unlink()
        with self.assertRaisesRegex(ValueError, "marked"):
            proof.validate_paths(self.repo, self.root / "scratch", self.target)
        marker.write_text(json.dumps({"workspace": str(self.root)}), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "workspace"):
            proof.validate_paths(self.repo, self.root / "scratch", self.target)
        marker.write_text("[]", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "workspace"):
            proof.validate_paths(self.repo, self.root / "scratch", self.target)

    def archive(self, name="opi-ai-0.8.3/Cargo.toml", link=False):
        path = self.root / "package.crate"
        with tarfile.open(path, "w:gz") as archive:
            entry = tarfile.TarInfo(name)
            if link:
                entry.type = tarfile.SYMTYPE
                entry.linkname = "../../outside"
                archive.addfile(entry)
            else:
                data = b"[package]\nname = 'opi-ai'\nversion = '0.8.3'\n"
                entry.size = len(data)
                archive.addfile(entry, io.BytesIO(data))
        return path

    def test_packaged_manifest_is_extracted_from_expected_package(self):
        destination = self.root / "package"
        result = proof.unpack_package(self.archive(), destination, "opi-ai-0.8.3")
        self.assertTrue((result / "Cargo.toml").is_file())

    def test_archive_traversal_wrong_root_and_links_are_refused(self):
        for name, link in [("../outside", False), ("other/Cargo.toml", False),
                           ("opi-ai-0.8.3/link", True)]:
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "package member"):
                proof.unpack_package(self.archive(name, link), self.root / "package", "opi-ai-0.8.3")
        self.assertFalse((self.root / "outside").exists())

    def graph(self, default=False):
        package = self.root / "package" / "opi-ai-0.8.3"
        packages = [
            {"id": "opi", "name": "opi-ai", "version": "0.8.3", "source": None,
             "manifest_path": str(package / "Cargo.toml")},
            {"id": "consumer", "name": "opi-sdk-conformance-consumer", "version": "0.0.0",
             "source": None, "manifest_path": str(self.root / "consumer" / "Cargo.toml")},
        ]
        for name, version in [("regress", "0.12.0"), ("url", "2.5.8")]:
            packages.append({"id": name, "name": name, "version": version,
                             "source": "registry+https://github.com/rust-lang/crates.io-index"})
        features = ["std", "utf16", "prohibit-unsafe"]
        if default:
            features += ["default", "backend-pikevm"]
        return {"packages": packages, "resolve": {"nodes": [
            {"id": "regress", "features": features}, {"id": "url", "features": ["std"]}
        ]}}, package

    def test_registry_graph_and_both_caller_feature_profiles_are_recorded(self):
        for default in [False, True]:
            metadata, package = self.graph(default)
            result = proof.check_graph(metadata, package, default)
            self.assertEqual(result["regress"]["version"], "0.12.0")
            self.assertEqual("backend-pikevm" in result["regress"]["features"], default)

    def test_vendor_or_non_registry_backend_is_refused(self):
        for mutation in ["vendor", "path"]:
            metadata, package = self.graph()
            if mutation == "vendor":
                metadata["packages"][2]["name"] = "opi-vendor-regress"
            else:
                metadata["packages"][2]["source"] = None
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                proof.check_graph(metadata, package, False)

    def test_missing_feature_wrong_version_and_source_checkout_are_refused(self):
        for mutation in ["feature", "version", "checkout"]:
            metadata, package = self.graph()
            if mutation == "feature":
                metadata["resolve"]["nodes"][0]["features"].remove("utf16")
            elif mutation == "version":
                metadata["packages"][2]["version"] = "0.13.0"
            else:
                metadata["packages"][0]["manifest_path"] = str(self.repo / "Cargo.toml")
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                proof.check_graph(metadata, package, False)

    def test_leaf_overlay_keeps_historical_oracle_and_limits_data_differences(self):
        harness = Path(__file__).parent
        frozen = json.loads((harness / "frozen/leaf-expected.json").read_text(encoding="utf-8"))
        overlay = json.loads((harness / "current-profile.json").read_text(encoding="utf-8"))
        old = {case["id"]: case["result"] for case in frozen}
        self.assertEqual(set(overlay["leaf_overrides"]), {"url-23", "url-28"})
        for case_id, update in overlay["leaf_overrides"].items():
            self.assertIs(old[case_id], False)
            self.assertIs(update["result"], True)
            self.assertTrue(update["reason"])

    def test_cli_rejects_existing_scratch_before_running_cargo(self):
        sentinel = self.root / "keep.txt"
        sentinel.write_text("keep", encoding="utf-8")
        env = dict(os.environ)
        env.pop("ICU4X_DATA_DIR", None)
        result = subprocess.run(
            [sys.executable, str(Path(__file__).with_name("run_early_package_proof.py")),
             "--scratch", str(self.root), "--target-dir", str(self.target)],
            capture_output=True, text=True, encoding="utf-8", check=False, env=env,
        )
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertIn("already exists", result.stderr)
        self.assertEqual(sentinel.read_text(encoding="utf-8"), "keep")

    def test_set_icu_data_is_rejected_before_scratch_or_subprocess(self):
        data = self.root / "custom-data"
        data.mkdir()
        for index, value in enumerate(["", str(data), str(self.root / "missing-data")]):
            scratch = self.root / f"scratch-{index}"
            stderr = io.StringIO()
            with self.subTest(value=value), \
                    patch.dict(proof.os.environ, {"ICU4X_DATA_DIR": value}), \
                    patch.object(proof, "REPO", self.repo), \
                    patch.object(proof.subprocess, "run", return_value=SimpleNamespace(
                        returncode=1, stdout="", stderr="unexpected subprocess")) as run, \
                    patch.object(sys, "argv", ["proof", "--scratch", str(scratch),
                                               "--target-dir", str(self.target)]), \
                    patch("sys.stderr", stderr), patch("sys.stdout", io.StringIO()):
                with self.assertRaises(SystemExit) as rejected:
                    proof.main()
                self.assertEqual(rejected.exception.code, 2)
                self.assertIn("ICU4X_DATA_DIR must be unset", stderr.getvalue())
                self.assertFalse(scratch.exists())
                run.assert_not_called()

    def test_cli_rejects_empty_or_nonempty_icu_data_before_side_effects(self):
        # A broken preflight must not launch real Cargo during the regression.
        bootstrap = (
            "import runpy, sys\n"
            "from unittest.mock import patch\n"
            "script = sys.argv.pop(1)\n"
            "with patch('subprocess.run', side_effect=AssertionError('unexpected subprocess')):\n"
            "    runpy.run_path(script, run_name='__main__')\n"
        )
        (self.target / ".opi-cargo-cache.json").write_text(
            json.dumps({"workspace": str(proof.REPO)}), encoding="utf-8"
        )
        data = self.root / "custom-data"
        data.mkdir()
        for index, value in enumerate(["", str(data), str(self.root / "missing-data")]):
            scratch = self.root / f"cli-scratch-{index}"
            env = dict(os.environ, ICU4X_DATA_DIR=value)
            with self.subTest(value=value):
                result = subprocess.run(
                    [sys.executable, "-B", "-c", bootstrap,
                     str(Path(__file__).with_name("run_early_package_proof.py")),
                     "--scratch", str(scratch), "--target-dir", str(self.target)],
                    capture_output=True, text=True, encoding="utf-8", env=env, check=False,
                )
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("ICU4X_DATA_DIR must be unset", result.stderr)
                self.assertFalse(scratch.exists())

    def test_report_cannot_hide_a_missing_duplicate_or_failed_check(self):
        checks = [{"name": str(index), "passed": True} for index in range(73)]
        proof.check_report({"verdict":"PASS", "total":73, "failed":0, "checks":checks})
        for changed in [checks[:-1], checks[:-1] + [checks[0]],
                        checks[:-1] + [{"name":"72", "passed":False}]]:
            with self.subTest(checks=changed[-1:]), self.assertRaises(ValueError):
                proof.check_report({"verdict":"PASS", "total":73, "failed":0, "checks":changed})
        for malformed in [[], {"checks": {}}, {"checks": [{}]}, {"checks": [None]}]:
            with self.subTest(malformed=malformed), self.assertRaises(ValueError):
                proof.check_report(malformed)

    def test_failed_cargo_step_records_failure_and_releases_shared_cache_lease(self):
        (self.repo / "Cargo.toml").write_text(
            '[workspace.package]\nversion = "0.8.3"\n', encoding="utf-8"
        )
        scratch = self.root / "scratch"
        commands = []

        def fake_run(command, **kwargs):
            commands.append(command)
            failure = command[:2] == ["cargo", "package"]
            return SimpleNamespace(returncode=1 if failure else 0, stdout="", stderr="expected failure")

        with patch.object(proof, "REPO", self.repo), patch.object(proof.subprocess, "run", fake_run), \
                patch.object(sys, "argv", ["proof", "--scratch", str(scratch), "--target-dir", str(self.target)]), \
                patch("sys.stdout", io.StringIO()), patch.dict(proof.os.environ):
            proof.os.environ.pop("ICU4X_DATA_DIR", None)
            self.assertEqual(proof.main(), 1)
        evidence = json.loads((scratch / "evidence.json").read_text(encoding="utf-8"))
        self.assertEqual(evidence["verdict"], "FAIL")
        self.assertEqual(evidence["profiles"], [])
        self.assertEqual([step["name"] for step in evidence["steps"]], ["lease-start", "package", "lease-end"])
        self.assertIn("end", commands[-1])
        self.assertTrue((scratch / "package.stderr").is_file())


if __name__ == "__main__":
    unittest.main()
