import copy
import unittest

from compare_chat_argument_preview import compare, json_equal


class PreviewComparisonTests(unittest.TestCase):
    def setUp(self):
        self.fixture = [{"name": "literal", "pieces": ['[false'], "final": [False]}]
        self.before = {"cases": [{"name": "literal", "previews": [{"raw": '[false', "preview": []}],
                                  "final": [False], "request": {}}]}
        self.after = copy.deepcopy(self.before)
        self.after["cases"][0]["previews"][0]["preview"] = [False]
        durable = {"progress": [[False]], "final": [False]}
        self.pi = {"cases": [{"name": "literal", "previews": [[False]], "utility": [[False]],
                              "final": [False], "request": {}, "durable": {"pi": durable, "opi": copy.deepcopy(durable)}}]}

    def check(self):
        return compare(self.fixture, self.pi, self.before, self.after, {"literal": [0]})

    def test_literal_repair_is_not_full_alignment(self):
        self.assertEqual(self.check()["repaired_checkpoints"], 1)
        self.assertFalse(self.check()["strict_alignment"])

    def test_missing_case_rejected(self):
        self.after["cases"] = []
        with self.assertRaisesRegex(ValueError, "inventory"):
            self.check()

    def test_missing_checkpoint_rejected(self):
        self.after["cases"][0]["previews"] = []
        with self.assertRaisesRegex(ValueError, "checkpoint"):
            self.check()

    def test_raw_loss_rejected(self):
        self.after["cases"][0]["previews"][0]["raw"] = "false"
        with self.assertRaisesRegex(ValueError, "raw argument"):
            self.check()

    def test_unrepaired_literal_rejected(self):
        self.after = copy.deepcopy(self.before)
        with self.assertRaisesRegex(ValueError, "expected literal repair"):
            self.check()

    def test_one_fixed_point_cannot_hide_an_unfixed_point(self):
        self.fixture.append({"name": "second", "pieces": ['[null'], "final": [None]})
        unchanged = {"name": "second", "previews": [{"raw": '[null', "preview": []}], "final": [None], "request": {}}
        self.before["cases"].append(copy.deepcopy(unchanged))
        self.after["cases"].append(copy.deepcopy(unchanged))
        durable = {"progress": [[None]], "final": [None]}
        self.pi["cases"].append({"name": "second", "previews": [[None]], "utility": [[None]],
                                 "final": [None], "request": {}, "durable": {"pi": durable, "opi": durable}})
        with self.assertRaisesRegex(ValueError, "expected literal repair"):
            compare(self.fixture, self.pi, self.before, self.after, {"literal": [0], "second": [0]})

    def test_number_cannot_replace_boolean(self):
        self.after["cases"][0]["previews"][0]["preview"] = [0]
        with self.assertRaisesRegex(ValueError, "literal repair"):
            self.check()

    def test_json_equality_preserves_boolean_type(self):
        self.assertTrue(json_equal({"n": [125.0]}, {"n": [125]}))
        for a, b in [(False, 0), (True, 1), ([False], [0]), ({"x": True}, {"x": 1})]:
            self.assertFalse(json_equal(a, b))

    def test_final_drift_rejected(self):
        self.after["cases"][0]["final"] = [True]
        with self.assertRaisesRegex(ValueError, "final arguments"):
            self.check()

    def test_consumer_drift_rejected(self):
        self.pi["cases"][0]["durable"]["opi"]["progress"] = [[]]
        with self.assertRaisesRegex(ValueError, "Durable"):
            self.check()

    def test_wrong_repair_rejected(self):
        self.after["cases"][0]["previews"][0]["preview"] = [True]
        with self.assertRaisesRegex(ValueError, "literal repair"):
            self.check()


if __name__ == "__main__":
    unittest.main()
