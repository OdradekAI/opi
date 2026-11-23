from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import check_test_inventory as inventory


class InventoryTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        tests = self.root / "tests"
        tests.mkdir()
        source = tests / "shared.rs"
        source.touch()
        self.targets = [
            {"name": "core", "src_path": str(source)},
            {"name": "optional", "src_path": str(source), "required-features": ["optional"]},
        ]
        self.file = self.root / "targets.tsv"
        self.rows = [
            "core\tprovider\tpublic-contract\tin-process\tCore behavior\t-",
            "optional\tprovider\tfeature-profile\tin-process\tOptional behavior\t-",
        ]
        self.write_inventory()

    def write_inventory(self):
        self.file.write_text("\n".join(["\t".join(inventory.FIELDS), *self.rows]) + "\n",
                             encoding="utf-8")

    def resolve(self):
        return inventory.resolve_inventory(
            self.file, self.targets, {"core", "optional"}, self.root, self.root
        )

    def test_shared_source_keeps_independent_feature_targets(self):
        rows = self.resolve()
        self.assertEqual([r["source"] for r in rows], ["tests/shared.rs"] * 2)
        self.assertEqual([r["required-features"] for r in rows], ["-", "optional"])

    def test_detects_unowned_file_and_implicit_target(self):
        unowned = self.root / "tests/unowned.rs"
        unowned.touch()
        with self.assertRaisesRegex(ValueError, "unowned root"):
            self.resolve()
        self.targets.append({"name": "unowned", "src_path": str(unowned)})
        with self.assertRaisesRegex(ValueError, "explicit Cargo declarations"):
            self.resolve()

    def test_detects_missing_duplicate_and_stale_binding(self):
        optional = self.rows.pop()
        self.write_inventory()
        with self.assertRaisesRegex(ValueError, "inventory targets: missing .*optional"):
            self.resolve()
        self.rows.append(self.rows[0])
        self.write_inventory()
        with self.assertRaisesRegex(ValueError, "duplicate inventory target"):
            self.resolve()
        self.rows[-1] = optional.removesuffix("-") + "archive@missing.py"
        self.write_inventory()
        with self.assertRaisesRegex(ValueError, "invalid runner"):
            self.resolve()

    def test_cli_rejects_inventory_missing_real_targets(self):
        result = subprocess.run(
            [sys.executable, inventory.__file__, "--inventory", str(self.file)],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("inventory targets: missing", result.stderr)


if __name__ == "__main__":
    unittest.main()
