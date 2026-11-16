"""Focused tests for the inventory and drift checker, not product tests."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import freeze_baseline
import source_inventory as scan
import verify_phase1

PARSER_PATH = None
REPO = None
SCRATCH = None


class InventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        sys.path.insert(0, str(PARSER_PATH))
        from tree_sitter import Language, Parser
        import tree_sitter_typescript
        import tree_sitter_rust
        cls.ts = Parser(Language(tree_sitter_typescript.language_typescript()))
        cls.rs = Parser(Language(tree_sitter_rust.language()))

    def module(self, raw, path):
        scan.SOURCE_BYTES = raw
        return scan.ts_module(self.ts.parse(raw), path)

    def test_alias_import_reexport_and_type_only(self):
        modules = {
            "a.ts": self.module(b'export class A { private hidden(){} public shown(){} }', "a.ts"),
            "b.ts": self.module(b'import { A as B } from "./a.ts"; export type {B};', "b.ts"),
            "c.ts": self.module(b'export * from "./b.ts";', "c.ts")}
        resolved, errors = scan.resolve_exports(modules)
        self.assertEqual(errors, [])
        self.assertEqual(resolved["c.ts"]["B"][0]["classification"], "type-only")
        self.assertEqual([m["name"] for m in resolved["a.ts"]["A"][0]["members"]], ["shown"])

    def test_missing_reexport_is_not_silent(self):
        module = self.module(b'export {missing} from "./missing.ts";', "a.ts")
        _, errors = scan.resolve_exports({"a.ts": module})
        self.assertEqual(len(errors), 1)
        self.assertEqual(errors[0]["reason"], "named target missing")

    def test_cycle_requires_review(self):
        modules = {"a.ts": self.module(b'export * from "./b.ts"; export const A=1;', "a.ts"),
                   "b.ts": self.module(b'export * from "./a.ts"; export const B=1;', "b.ts")}
        _, errors = scan.resolve_exports(modules)
        self.assertTrue(any("cyclic" in e["reason"] for e in errors))

    def test_parent_skip_and_todo(self):
        raw = b'describe.skipIf(!key)("live",()=>{ it("call",()=>{}); it.todo("later"); });'
        scan.SOURCE_BYTES = raw
        cases = scan.ts_cases(self.ts.parse(raw), "a.test.ts")
        self.assertEqual(len(cases), 2)
        self.assertTrue(all(c["skip_or_conditional"] for c in cases))
        self.assertIn("skipIf", cases[0]["suite_forms"][0])

    def test_parameterized_declaration_not_expanded_case_count(self):
        raw = b'describe("group",()=>{ it.each([1,2])("value %s",()=>{}); it.skip("skip",()=>{}); });'
        scan.SOURCE_BYTES = raw
        cases = scan.ts_cases(self.ts.parse(raw), "a.test.ts")
        self.assertEqual(len(cases), 2)
        self.assertTrue(cases[0]["parameterized"])
        self.assertTrue(cases[1]["skip_or_conditional"])
        self.assertEqual(cases[0]["suite"], ['"group"'])

    def test_rust_attributes_and_private_exclusion(self):
        raw = b'pub fn visible(){} fn hidden(){} #[tokio::test] #[ignore] async fn example(){}'
        scan.SOURCE_BYTES = raw
        public, tests = scan.rust_records(self.rs.parse(raw), "a.rs")
        self.assertEqual([p["name"] for p in public], ["visible"])
        self.assertEqual([t["name"] for t in tests], ["example"])
        self.assertIn("#[ignore]", tests[0]["attributes"])

    def test_parse_buffer_does_not_change_recorded_bytes(self):
        original = b'#[test] fn example(){ f(&raw); }'
        scan.SOURCE_BYTES = original
        _, tests = scan.rust_records(self.rs.parse(original.replace(b"&raw", b"&r_w")), "a.rs")
        self.assertEqual(tests[0]["body_sha256"], scan.sha(b'fn example(){ f(&raw); }'))


class VerificationTests(unittest.TestCase):
    def test_current_inventory_content_drift_is_rejected(self):
        inventory_path = REPO / "docs/iterations/opi-ai-1-5-alignment/evidence/phase1-current-source-inventory-20261115.json"
        inventory = json.loads(inventory_path.read_text(encoding="utf-8"))
        inventory["files"][0]["sha256"] = "0" * 64
        errors = []
        verify_phase1.check_current_inventory(REPO, inventory, errors)
        self.assertTrue(any("current inventory content drift" in e for e in errors))

    def test_cli_current_snapshot(self):
        baseline = REPO / "docs/iterations/opi-ai-1-5-alignment/evidence/phase1-current-baseline-20261115.json"
        inventory = REPO / "docs/iterations/opi-ai-1-5-alignment/evidence/phase1-current-source-inventory-20261115.json"
        result = subprocess.run([
            sys.executable, str(Path(__file__).with_name("verify_phase1.py")),
            "--repo", str(REPO), "--mode", "current",
            "--baseline", str(baseline), "--inventory", str(inventory),
        ], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("PASS: current source identities", result.stdout)

    def test_source_drift(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as folder:
            root = Path(folder)
            (root / "a.txt").write_bytes(b"original")
            manifest = freeze_baseline.manifest(root, ["a.txt"])
            errors = []
            verify_phase1.check_manifest(root, manifest, errors, ["a.txt"])
            self.assertEqual(errors, [])
            (root / "a.txt").write_bytes(b"changed")
            verify_phase1.check_manifest(root, manifest, errors, ["a.txt"])
            self.assertTrue(any("source drift" in e for e in errors))

    def test_added_file_and_duplicate_ids(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as folder:
            root = Path(folder)
            (root / "a").write_bytes(b"a")
            manifest = freeze_baseline.manifest(root, ["a"])
            errors = []
            verify_phase1.check_manifest(root, manifest, errors, ["a", "new"])
            verify_phase1.unique([{"id": "same"}, {"id": "same"}], "id", "fixture", errors)
            self.assertEqual(len(errors), 2)

    def test_cli_refuses_overwrite(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as folder:
            target = Path(folder) / "baseline.json"
            target.write_bytes(b"unchanged")
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("freeze_baseline.py")),
                                     "--repo", str(REPO), "--output", str(target)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 2)
            self.assertIn("output already exists", result.stderr)
            self.assertEqual(target.read_bytes(), b"unchanged")

    def test_cli_detects_corrupt_baseline(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as folder:
            source = Path(__file__).resolve().parent.parent / "evidence/phase1-baseline.json"
            baseline = json.loads(source.read_text(encoding="utf-8"))
            baseline["upstream"]["manifest"]["files"][0]["sha256"] = "0" * 64
            target = Path(folder) / "corrupt.json"
            target.write_text(json.dumps(baseline), encoding="utf-8")
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("verify_phase1.py")),
                                     "--repo", str(REPO), "--baseline", str(target)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("manifest aggregate mismatch", result.stdout)
            self.assertIn("source drift", result.stdout)

    def test_cli_rejects_deleted_declaration(self):
        with tempfile.TemporaryDirectory(dir=SCRATCH) as folder:
            source = Path(__file__).resolve().parent.parent / "contract-index.json"
            index = json.loads(source.read_text(encoding="utf-8"))
            index["test_declarations"].pop()
            target = Path(folder) / "incomplete.json"
            target.write_text(json.dumps(index), encoding="utf-8")
            result = subprocess.run([sys.executable, str(Path(__file__).with_name("verify_phase1.py")),
                                     "--repo", str(REPO), "--index", str(target)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            self.assertIn("declaration-set drift: test_declarations", result.stdout)

    def test_missing_navigation_is_rejected(self):
        source = Path(__file__).resolve().parent.parent / "contract-index.json"
        for collection in ("entrypoints", "consumers", "models_methods", "provider_factories"):
            with self.subTest(collection=collection):
                index = json.loads(source.read_text(encoding="utf-8"))
                index[collection] = []
                errors = []
                verify_phase1.check_index(REPO, index, errors)
                self.assertIn(f"navigation-set drift: {collection}", errors)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--parser-path", type=Path, required=True)
    parser.add_argument("--repo", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    PARSER_PATH, REPO, SCRATCH = args.parser_path.resolve(), args.repo.resolve(), args.scratch.resolve()
    SCRATCH.mkdir(parents=True, exist_ok=True)
    unittest.main(argv=[sys.argv[0]], verbosity=2)
