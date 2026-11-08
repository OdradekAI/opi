import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).parent))
import restore_pinned_catalog as restore


def baseline_for(root, names):
    return {"upstream": {"manifest": {"sha256": "fixture", "files": [
        {"path": name, "sha256": hashlib.sha256((root / name).read_bytes()).hexdigest()}
        for name in names
    ]}}}


class RestorePinnedCatalogTests(unittest.TestCase):
    def test_copy_preserves_source_and_refuses_reuse_or_nonlock_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            (source / "package-lock.json").write_bytes(b"frozen lock")
            (source / "entry.ts").write_bytes(b"frozen source\r\n")
            baseline = baseline_for(source, ["package-lock.json", "entry.ts"])
            (source / "package-lock.json").write_bytes(b"supplied modified lock")
            candidate = root / "copy"
            self.assertEqual(restore.copy_sources(source, candidate, baseline), 2)
            for name in ("package-lock.json", "entry.ts"):
                self.assertEqual((source / name).read_bytes(), (candidate / name).read_bytes())
            self.assertEqual(restore.source_comparison(candidate, baseline)["changes"][0]["path"], "package-lock.json")
            with self.assertRaises(FileExistsError):
                restore.copy_sources(source, candidate, baseline)
            (source / "entry.ts").write_bytes(b"modified source")
            with self.assertRaisesRegex(ValueError, "unexpected source drift"):
                restore.copy_sources(source, root / "second", baseline)
            self.assertFalse((root / "second").exists())

    def test_destinations_cannot_overlap_or_overwrite(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            output = root / "proof.json"
            candidate = root / "candidate"
            restore.check_destinations(source, candidate, output)
            for destination in (source, source / "candidate", root):
                with self.subTest(destination=destination), self.assertRaises((ValueError, FileExistsError)):
                    restore.check_destinations(source, destination, output)
            with self.assertRaisesRegex(ValueError, "separate"):
                restore.check_destinations(source, candidate, candidate / "proof.json")
            output.write_bytes(b"historic proof")
            with self.assertRaises(FileExistsError):
                restore.check_destinations(source, candidate, output)
            self.assertEqual(output.read_bytes(), b"historic proof")

    def test_dangling_directory_links_are_rejected_before_inputs_or_commands(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            target = root / "target"
            target.mkdir()
            dangling = root / "dangling"
            restore.link_directory(target, dangling)
            target.rmdir()
            self.assertFalse(dangling.exists())
            for candidate, output in ((dangling, root / "proof.json"), (root / "candidate", dangling)):
                with self.subTest(candidate=candidate, output=output):
                    with self.assertRaisesRegex(FileExistsError, "refusing to overwrite"):
                        restore.check_destinations(source, candidate, output)
                    result = subprocess.run([sys.executable, "-B", str(Path(restore.__file__)), "--repo", directory,
                                             "--candidate", str(candidate), "--output", str(output)],
                                            capture_output=True, text=True)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn("refusing to overwrite", result.stderr)
            self.assertFalse((root / "candidate").exists())
            self.assertFalse((root / "proof.json").exists())

    def test_baseline_paths_cannot_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "entry.ts").write_bytes(b"source")
            # Windows can resolve this alias back inside source while copying
            # the same lexical relative path would escape the candidate root.
            alias = f"..\\{root.name}\\entry.ts"
            for name in ("../outside", "/absolute", "C:/absolute", "a/../escape", "./relative", "missing", alias):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    restore.checked_source(root, name)

    def test_dependency_links_use_candidate_workspace_and_nested_versions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, candidate = root / "source", root / "candidate"
            for relative in ("node_modules/probe", "node_modules/@earendil-works/pi-schema", "packages/ai/node_modules/probe"):
                (source / relative).mkdir(parents=True)
            with patch.object(restore, "link_directory") as link:
                links = restore.share_dependencies(source, candidate)
            self.assertEqual(link.call_count, 3)
            by_path = {row["path"]: row["target"] for row in links}
            self.assertEqual(by_path["node_modules/@earendil-works/pi-schema"], str(candidate / "packages/schema"))
            self.assertEqual(by_path["packages/ai/node_modules/probe"], str(source / "packages/ai/node_modules/probe"))

    def test_directory_link_treats_shell_characters_as_literal_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source & literal %value%"
            source.mkdir()
            (source / "sentinel").write_bytes(b"source retained")
            link = root / "candidate & literal %value%"
            restore.link_directory(source, link)
            self.assertEqual((link / "sentinel").read_bytes(), b"source retained")
            with self.assertRaises(OSError):
                restore.link_directory(source, link)
            self.assertEqual((source / "sentinel").read_bytes(), b"source retained")

    def test_failed_build_keeps_candidate_and_evidence_and_checks_source_drift(self):
        for mutate in (False, True):
            with self.subTest(mutate=mutate), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                source = root / "source"
                (source / "nix").mkdir(parents=True)
                catalog = root / "catalog.json"
                catalog.write_bytes(b'{"probe":[]}')
                revision = "sha256-" + hashlib.sha256(catalog.read_bytes()).hexdigest()
                (source / "nix/model-catalog.json").write_text(json.dumps({"revision": revision}))
                (source / "package.json").write_text('{"devDependencies":{"typescript":"7.0.2"}}')
                compiler = source / "node_modules/typescript"
                (compiler / "bin").mkdir(parents=True)
                (compiler / "bin/tsc").write_bytes(b"fixture compiler")
                (compiler / "package.json").write_text('{"version":"7.0.2"}')
                baseline = root / "baseline.json"
                baseline.write_text(json.dumps(baseline_for(source, ["nix/model-catalog.json", "package.json"])))

                def command(args, cwd, *, isolated):
                    self.assertTrue(isolated)
                    failed = "-p" in args and Path(args[-1]).parent.name == "ai"
                    if failed and mutate:
                        (source / "package.json").write_bytes(b"unexpected mutation")
                    return {"command": args, "status": "error" if failed else "ok", "exit_code": 9 if failed else 0,
                            "stdout": "{}", "stderr": "compiler-negative-marker" if failed else ""}

                output = root / "proof.json"
                candidate = root / "candidate"
                with patch.object(restore, "command_result", side_effect=command), patch.object(restore, "share_dependencies", return_value=[]):
                    record, code = restore.restore(root, source, candidate, catalog, baseline, output)
                self.assertEqual(code, 1)
                self.assertEqual(record["status"], "failed")
                self.assertTrue(candidate.is_dir())
                self.assertEqual(json.loads(output.read_text(encoding="utf-8")), record)
                self.assertEqual(record["supplied_unchanged"], not mutate)
                self.assertIn("all files in shared node_modules", record["supplied_unchanged_scope"]["excluded"])
                self.assertEqual(record["commands"][-1]["stderr"], "compiler-negative-marker")
                self.assertIn("identity changed" if mutate else "command failed", record["error"])

    def test_pin_mismatch_rejected_before_candidate_or_process(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            (source / "nix").mkdir(parents=True)
            (source / "nix/model-catalog.json").write_text(json.dumps({"revision": "sha256-" + "0" * 64}))
            baseline = root / "baseline.json"
            baseline.write_text(json.dumps(baseline_for(source, ["nix/model-catalog.json"])))
            catalog = root / "catalog.json"
            catalog.write_bytes(b'{"probe":[]}')
            with patch.object(restore, "command_result") as command:
                with self.assertRaisesRegex(ValueError, "pinned catalog rejected"):
                    restore.restore(root, source, root / "candidate", catalog, baseline, root / "proof.json")
            command.assert_not_called()
            self.assertFalse((root / "candidate").exists())
            self.assertFalse((root / "proof.json").exists())

    def test_cli_refuses_existing_output_before_loading_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "proof.json"
            output.write_bytes(b"unchanged")
            result = subprocess.run([sys.executable, "-B", str(Path(restore.__file__)), "--repo", directory,
                                     "--candidate", str(root / "candidate"), "--output", str(output)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("refusing to overwrite", result.stderr)
            self.assertEqual(output.read_bytes(), b"unchanged")
            self.assertFalse((root / "candidate").exists())


if __name__ == "__main__":
    unittest.main()
