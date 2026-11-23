import csv
from pathlib import Path
import tempfile
import unittest

import check_support_inventory as support_inventory
import check_test_inventory as test_inventory


class SupportInventoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = test_inventory.load_inventory()

    def copy_inventory(self):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / "OWNERS.tsv"
        path.write_text(support_inventory.DEFAULT_INVENTORY.read_text(encoding="utf-8"),
                        encoding="utf-8")
        return path

    def test_declares_every_support_helper_and_actual_consumer(self):
        rows = support_inventory.resolve_inventory(rows=self.rows)
        self.assertEqual(len(rows), len(list(support_inventory.SUPPORT.rglob("*.rs"))))
        self.assertEqual(sum(len(support_inventory.parse_list(row["consumers"])) for row in rows), 60)

    def test_rejects_stale_consumer(self):
        path = self.copy_inventory()
        records = []
        with path.open(encoding="utf-8", newline="") as stream:
            reader = csv.DictReader(stream, delimiter="\t")
            records.extend(reader)
        for row in records:
            if row["helper"] == "pending_auth.rs":
                row["consumers"] = "classifier_contract,deferred_contract"
        with path.open("w", encoding="utf-8", newline="") as stream:
            writer = csv.DictWriter(stream, support_inventory.FIELDS, delimiter="\t",
                                    lineterminator="\n")
            writer.writeheader()
            writer.writerows(records)
        with self.assertRaisesRegex(ValueError, "consumers for pending_auth.rs"):
            support_inventory.resolve_inventory(path, self.rows)

    def test_rejects_unowned_support_file(self):
        path = self.copy_inventory()
        with path.open("a", encoding="utf-8") as stream:
            stream.write("orphan.rs\tstable-tool\ttest support\tpublic-contract\t"
                         "in-process\t-\tclassifier_contract\n")
        with self.assertRaisesRegex(ValueError, "missing support helper"):
            support_inventory.resolve_inventory(path, self.rows)


if __name__ == "__main__":
    unittest.main()
