"""Mutation controls against the frozen paired receipt, with portable CLI inputs."""
import copy
import tempfile
import json
import subprocess
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

import compare_incremental_argument_preview as m
from common import FIXTURE_DIR

REPO = Path(__file__).resolve().parents[3]
MODULE = Path(m.__file__).resolve()
FIXTURE_PATH = REPO / "crates/opi-ai/tests/data/incremental-argument-preview.json"
PAIR = m.load_trace(FIXTURE_DIR / "phase5-final-numbers-differential.json")["replays"][0]
F = m.load_trace(FIXTURE_PATH)
P, R = PAIR["interface_pi"]["trace"], PAIR["interface_rust"]["trace"]
CF = m.load_trace(REPO / "crates/opi-ai/tests/data/chat-argument-preview.json")
CP, CO, CR = (PAIR[side]["trace"] for side in ("pi", "old", "rust"))



def case(trace, name):
    return next(item for item in trace["cases"] if item["name"] == name)


def sync_retained(trace):
    for item in trace["cases"]:
        item["retained"] = [copy.deepcopy(row["preview"]) for row in item["rows"]]


class Controls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        global SCRATCH, PI_PATH, RUST_PATH
        temporary = tempfile.TemporaryDirectory(prefix="opi-preview-comparator-")
        cls.addClassCleanup(temporary.cleanup)
        SCRATCH = Path(temporary.name)
        PI_PATH, RUST_PATH = SCRATCH / "pi.json", SCRATCH / "rust.json"
        for path, trace in ((PI_PATH, P), (RUST_PATH, R)):
            path.write_text(json.dumps(trace, ensure_ascii=True), encoding="utf-8")

    def test_trace_loader_rejects_ambiguous_values(self):
        path = SCRATCH / "invalid.json"
        for text in ['{"a":1,"a":2}', '{"n":1e400}', '{"n":NaN}', '{"n":Infinity}']:
            with self.subTest(text=text):
                path.write_text(text, encoding="utf-8")
                with self.assertRaises(ValueError):
                    m.load_trace(path)

    def test_cli_positive_round_trip(self):
        output = SCRATCH / "comparison.json"
        run = subprocess.run([sys.executable, "-B", str(MODULE),
            "--fixture", str(FIXTURE_PATH), "--pi", str(PI_PATH),
            "--rust", str(RUST_PATH), "--output", str(output)], capture_output=True, text=True)
        self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
        report = m.load_trace(output)
        self.assertEqual(report["comparison"]["checkpoints"], 124)
        self.assertFalse(report["comparison"]["strict_alignment"])
        self.assertEqual(report["inputs"][str(PI_PATH.resolve())], m.file_hash(PI_PATH))

    def test_positive_interface(self):
        result = m.compare_interface(F, P, R)
        self.assertEqual((result["cases"], result["checkpoints"]), (23, 124))
        self.assertFalse(result["strict_alignment"])
        self.assertFalse(any(d["kind"] == "derived_final_value" for d in result["differences"]))

    def test_historical_final_rounding_is_now_rejected(self):
        old = m.load_trace(FIXTURE_DIR / "phase5-preview-boundaries-verified.json")["replays"][0]
        with self.assertRaisesRegex(ValueError, "final numeric/value decoding"):
            m.compare_interface(F, old["interface_pi"]["trace"], old["interface_rust"]["trace"])

    def test_equal_forged_final_numbers_are_rejected(self):
        for changes in ({"value":{"n":9.999999999999999e63}}, {"raw":'{"n":1e64}'}):
            p, r = copy.deepcopy((P, R))
            for trace in (p, r):
                case(trace, "long-scalar")["rows"][-1]["final"].update(changes)
            with self.assertRaises(ValueError):
                m.compare_interface(F, p, r)

    def test_numeric_final_is_bound_to_accumulated_text(self):
        f, p, r = copy.deepcopy((F, P, R))
        case(f, "long-scalar")["operations"][0]["text"] = " " + case(f, "long-scalar")["operations"][0]["text"]
        for trace in (p, r):
            for row in case(trace, "long-scalar")["rows"]:
                row["raw"] = " " + row["raw"]
        with self.assertRaisesRegex(ValueError, "controlled final raw differs"):
            m.compare_interface(f, p, r)

    def test_positive_chat(self):
        result = m.compare_chat(CF, CP, CO, CR)
        self.assertEqual((result["checkpoints"], result["closed_checkpoints"]), (65, 18))

    def test_numeric_callers_and_serialization(self):
        n, p, c = (PAIR[side]["trace"] for side in ("numeric", "numeric_pi", "callers"))
        self.assertEqual(m.compare_numeric(n, p, c)["finite_controls"], 10)
        mutations = [
            lambda n, p, c: n["final_numbers"].pop(),
            lambda n, p, c: n["final_numbers"][0].update(bits="4d384f03e93ff9f4"),
            lambda n, p, c: n["final_numbers"][0].update(serialized='{"n":9.999999999999999e63}'),
            lambda n, p, c: n["final_numbers"][8].update(serialized='{"n":0}'),
            lambda n, p, c: p["final_numbers"][0].update(raw='{"n":1e64}'),
            lambda n, p, c: p["agent"].pop(),
            lambda n, p, c: c["agent_final_number"].pop(),
            lambda n, p, c: c["bounded_agent_final_number"][0].update(bits="4d384f03e93ff9f4"),
            lambda n, p, c: c["session_final_number"].update(persisted='{"n":9.999999999999999e63,"password":"[REDACTED]"}'),
            lambda n, p, c: p["session"].update(executions_before_resume=2),
            lambda n, p, c: c["session_final_number"]["next_execution"][0].update(bits="4d384f03e93ff9f4"),
        ]
        for index, mutate in enumerate(mutations):
            with self.subTest(index=index):
                changed = copy.deepcopy((n, p, c))
                mutate(*changed)
                with self.assertRaises(ValueError):
                    m.compare_numeric(*changed)

    def test_interface_mutations(self):
        mutations = {
            "missing-case": lambda f, p, r: r["cases"].pop(),
            "missing-operation": lambda f, p, r: r["cases"][0]["rows"].pop(),
            "raw": lambda f, p, r: r["cases"][0]["rows"][0].update(raw="wrong"),
            "suffix": lambda f, p, r: case(r, "replace")["rows"][1].update(suffix="wrong"),
            "tool": lambda f, p, r: r["cases"][0]["rows"][0].update(tool=1),
            "float-tool": lambda f, p, r: r["cases"][0]["rows"][0].update(tool=0.0),
            "operation": lambda f, p, r: r["cases"][0]["rows"][0].update(op="finish"),
            "preview": lambda f, p, r: r["cases"][0]["rows"][0].update(preview={"wrong": True}),
            "retained": lambda f, p, r: r["cases"][0]["retained"][0].update(wrong=True),
            "key-order": lambda f, p, r: case(r, "key-order")["rows"][1]["key_order"][0]["keys"].reverse(),
            "missing-key-order": lambda f, p, r: r["cases"][0]["rows"][0].pop("key_order"),
            "negative-zero": lambda f, p, r: case(r, "numbers")["rows"][7].update(negative_zero_paths=[]),
            "fake-positive-zero": lambda f, p, r: case(r, "numbers")["rows"][7]["preview"].update(n=0.0),
            "non-finite-metadata": lambda f, p, r: case(p, "non-finite")["rows"][1]["non_finite"][0].update(kind="-Infinity"),
            "non-finite-extra-loss": lambda f, p, r: case(r, "non-finite")["rows"][1].update(preview={}),
            "non-finite-old-omission": lambda f, p, r: case(r, "non-finite")["rows"][1].update(preview={"z": 0}),
            "non-finite-shifted-array": lambda f, p, r: case(r, "non-finite-array")["rows"][3].update(preview={"a": [7, False]}),
            "non-finite-no-recovery": lambda f, p, r: case(r, "non-finite-duplicate")["rows"][3].update(preview={"n": 2}),
            "non-finite-cross-tool": lambda f, p, r: case(r, "non-finite-isolation")["rows"][1].update(preview={"ok": 1}),
            "non-finite-no-reset": lambda f, p, r: case(r, "non-finite-isolation")["rows"][2].update(preview={"ok": 1}),
            "non-finite-fake-position": lambda f, p, r: case(p, "non-finite-array")["rows"][2]["non_finite"][0].update(path=["a", 0]),
            "non-finite-renamed": lambda f, p, r: case(f, "non-finite").update(name="ordinary-overflow"),
            "non-finite-final": lambda f, p, r: case(r, "non-finite")["rows"][2]["final"].update(valid=True, value={}),
            "surrogate-forged-flag": lambda f, p, r: case(f, "empty").update(unicode_difference=True) or r["cases"][0]["rows"][0].update(preview={"wrong": True}),
            "surrogate-wrong-retained": lambda f, p, r: case(r, "unicode-pair")["rows"][1].update(preview={}),
            "surrogate-no-recovery": lambda f, p, r: case(r, "unicode-pair")["rows"][3].update(preview={"s": "x"}),
            "ordinary-final-invalid": lambda f, p, r: case(r, "empty")["rows"][1]["final"].update(valid=False, value=None),
            "final-raw": lambda f, p, r: case(r, "empty")["rows"][1]["final"].update(raw=""),
            "long-scalar-preview": lambda f, p, r: case(r, "long-scalar")["rows"][4].update(preview={"n": 9.999999999999999e63}),
            "long-scalar-other-value": lambda f, p, r: case(r, "long-scalar")["rows"][5]["final"].update(value={"n": 1}),
            "depth-wrong-preview": lambda f, p, r: case(r, "depth-boundary")["rows"][0].update(preview={"a": {}}),
            "depth-changed-fixture": lambda f, p, r: case(f, "depth-boundary")["operations"][0].update(text='{"a":0}'),
            "large-array-length": lambda f, p, r: r["large_array"]["lengths"].__setitem__(900, 1),
            "large-array-credit": lambda f, p, r: r["large_array"]["credit"].update(empty_append=571),
            "large-array-item-count": lambda f, p, r: f.update(large_array_items=1199),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                f, p, r = copy.deepcopy((F, P, R))
                mutate(f, p, r)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    m.compare_interface(f, p, r)

    def test_key_order_even_when_serialized_object_is_reordered(self):
        r = copy.deepcopy(R)
        row = case(r, "key-order")["rows"][1]
        row["preview"] = dict(reversed(list(row["preview"].items())))
        row["key_order"] = m.key_order(row["preview"])
        sync_retained(r)
        with self.assertRaisesRegex(ValueError, "preview key order"):
            m.compare_interface(F, P, r)

    def test_metadata_cannot_hide_same_negative_zero_loss_in_both_records(self):
        p, r = copy.deepcopy((P, R))
        for trace in (p, r):
            case(trace, "numbers")["rows"][7]["negative_zero_paths"] = []
        with self.assertRaisesRegex(ValueError, "Rust signed-zero metadata"):
            m.compare_interface(F, p, r)

    def test_duplicate_key_earlier_surrogate_is_detected(self):
        self.assertTrue(m.raw_lone_surrogate('{"a":"\\ud800","a":true}'))
        self.assertFalse(m.raw_lone_surrogate('{"a":"\\ud83d\\ude00","a":true}'))
        self.assertFalse(m.raw_lone_surrogate('{"a":"\\\\ud800","a":true}'))
        self.assertFalse(m.raw_lone_surrogate('{"a":"\\ud800"'))

    def test_non_finite_guard_includes_overwritten_members_and_excludes_strings(self):
        self.assertTrue(m.raw_non_finite('{"n":1e400,"n":false}'))
        self.assertTrue(m.raw_non_finite('{"a":[-1e400,true]}'))
        self.assertFalse(m.raw_non_finite('{"n":"1e400"}'))
        self.assertFalse(m.raw_non_finite('{"n":1e-400}'))
        self.assertFalse(m.raw_non_finite('{"n":1e400'))

    def test_overflow_policy_rejects_consistent_but_incorrect_snapshots(self):
        controls = [
            ("non-finite", 1, {"z": 0}),
            ("non-finite-array", 3, {"a": [7, False]}),
            ("non-finite-duplicate", 3, {"n": 2}),
            ("non-finite-isolation", 1, {"ok": 1}),
            ("non-finite-isolation", 2, {"ok": 1}),
        ]
        for name, index, value in controls:
            with self.subTest(name=name, index=index):
                r = copy.deepcopy(R)
                row = case(r, name)["rows"][index]
                row.update(preview=value, key_order=m.key_order(value),
                           negative_zero_paths=m.negative_zeros(value))
                sync_retained(r)
                with self.assertRaisesRegex(ValueError, "preview"):
                    m.compare_interface(F, P, r)

    def test_overflow_final_cannot_be_replaced_on_both_sides(self):
        p, r = copy.deepcopy((P, R))
        for trace in (p, r):
            case(trace, "non-finite")["rows"][2]["final"]["raw"] = '{"unrelated":1e400}'
        case(p, "non-finite")["rows"][2]["final"].update(
            value={"unrelated": None}, non_finite=[{"path": ["unrelated"], "kind": "Infinity"}])
        with self.assertRaisesRegex(ValueError, "final raw differs from accumulated"):
            m.compare_interface(F, p, r)

    def test_chat_mutations(self):
        mutations = {
            "lost-closure": lambda f, p, o, r: r["cases"][0]["previews"][1].update(preview={"a": None}),
            "legacy-drift": lambda f, p, o, r: r["cases"][0]["previews"][0].update(legacy_preview={"x": None}),
            "request": lambda f, p, o, r: r["cases"][0]["request"].update(model="wrong"),
            "final": lambda f, p, o, r: r["cases"][0].update(final={}),
            "durable": lambda f, p, o, r: p["cases"][0]["durable"]["opi"].update(final={}),
            "missing-raw": lambda f, p, o, r: r["cases"][0]["previews"][0].update(raw=""),
            "wrong-old": lambda f, p, o, r: o["cases"][0]["previews"][1].update(preview={}),
            "boolean-number": lambda f, p, o, r: r["cases"][-1]["previews"][4].update(preview=[1]),
            "missing-case": lambda f, p, o, r: r["cases"].pop(),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                f, p, o, r = copy.deepcopy((CF, CP, CO, CR))
                mutate(f, p, o, r)
                with self.assertRaises((ValueError, KeyError, TypeError)):
                    m.compare_chat(f, p, o, r)

    def test_unicode_hold_is_per_tool_and_replace_resets(self):
        f = {"cases": [{"name": "reset-control", "operations": [
            {"op": "append", "tool": 0, "text": '{"ok":1}'},
            {"op": "append", "tool": 1, "text": '{"s":"\\ud800"}'},
            {"op": "replace", "tool": 0, "text": '{"s":"\\ud800"}'},
        ]}], "large_array_items": 1200}
        def row(op, tool, raw, value):
            return {"op": op, "tool": tool, "raw": raw, "suffix": None, "preview": value,
                    "key_order": m.key_order(value), "negative_zero_paths": [], "non_finite": []}
        p = {"cases": [{"name": "reset-control", "rows": [
            row("append", 0, '{"ok":1}', {"ok": 1}),
            row("append", 1, '{"s":"\\ud800"}', {"s": "\ud800"}),
            row("replace", 0, '{"s":"\\ud800"}', {"s": "\ud800"}),
        ]}], "large_array": copy.deepcopy(P["large_array"])}
        r = copy.deepcopy(p)
        for index in (1, 2):
            r["cases"][0]["rows"][index]["preview"] = {}
            r["cases"][0]["rows"][index]["key_order"] = m.key_order({})
        sync_retained(p)
        sync_retained(r)
        m.compare_interface(f, p, r)
        r["cases"][0]["rows"][2]["preview"] = {"ok": 1}
        r["cases"][0]["rows"][2]["key_order"] = m.key_order({"ok": 1})
        sync_retained(r)
        with self.assertRaisesRegex(ValueError, "preview"):
            m.compare_interface(f, p, r)

    def test_cli_does_not_overwrite(self):
        before = m.file_hash(PI_PATH)
        run = subprocess.run([sys.executable, "-B", str(MODULE),
                              "--fixture", str(FIXTURE_PATH), "--pi", str(PI_PATH),
                              "--rust", str(RUST_PATH), "--output", str(PI_PATH)],
                             capture_output=True, text=True)
        self.assertEqual(run.returncode, 2)
        self.assertIn("output already exists", run.stdout)
        self.assertEqual(m.file_hash(PI_PATH), before)

    def test_cli_rechecks_inputs_before_write(self):
        output = SCRATCH / "must-not-be-written-input-drift.json"
        self.assertFalse(output.exists())
        hashes = [m.file_hash(path) for path in (FIXTURE_PATH, PI_PATH, RUST_PATH)]
        with patch.object(m, "file_hash", side_effect=hashes + ["drift"]):
            code = m.main(["--fixture", str(FIXTURE_PATH), "--pi", str(PI_PATH),
                           "--rust", str(RUST_PATH), "--output", str(output)])
        self.assertEqual(code, 2)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
