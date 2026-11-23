import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("sync.py").resolve()


def row(model_id, operation="chat", **values):
    return {"provider": "groq", "type": operation, "id": model_id, "api": "openai-completions", **values}


class CatalogSyncTest(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.sources = self.root / "sources"
        self.sources.mkdir()
        # A non-selected shard must stay byte-identical through a sync.
        self.other = b'[ {"id":"z","provider":"other","type":"chat","api":"other"} ]\r\n'
        (self.sources / "other.json").write_bytes(self.other)

    def release(self, name, rows):
        data = {"other": json.loads(self.other)}
        if rows:
            data["groq"] = rows
        path = self.root / (name + ".json")
        path.write_bytes(json.dumps(data).encode())
        revision = "sha256-" + hashlib.sha256(path.read_bytes()).hexdigest()
        pin = self.root / (name + "-pin.json")
        pin.write_text(json.dumps({"revision": revision}), encoding="utf-8")
        return path, pin

    def local(self, rows):
        self.authoring = (json.dumps(rows, indent=4) + "\n").encode()
        (self.sources / "groq.json").write_bytes(self.authoring)

    def cmd(self, old, new, *, sources=None, output="candidate", provider="groq"):
        return [sys.executable, "-B", str(SCRIPT),
                "--old-artifact", str(old[0]), "--old-pin", str(old[1]),
                "--new-artifact", str(new[0]), "--new-pin", str(new[1]),
                "--provider", provider, "--opi-sources", str(sources or self.sources),
                "--output-dir", str(self.root / output)]

    def run_cli(self, old, new, *, code=0, **kwargs):
        result = subprocess.run(self.cmd(old, new, **kwargs), capture_output=True)
        self.assertEqual(result.returncode, code, (result.stdout, result.stderr))
        return json.loads(result.stdout) if result.stdout else result.stderr.decode()

    def candidate(self, name):
        path = self.root / name / "providers"
        self.assertEqual((path / "other.json").read_bytes(), self.other)
        return json.loads((path / "groq.json").read_bytes())

    def test_two_upgrades_preserve_local_models_fields_and_deletions(self):
        shared = row("x", cost={"input": 1, "output": 2}, levels=[1, 2])
        old = self.release("v1", [shared, row("retiring"), row("locally-removed")])
        new = self.release("v2", [
            {**shared, "cost": {"input": 3, "output": 2}}, row("y"), row("locally-removed"),
        ])
        self.local([
            {**shared, "cost": {"input": 1, "output": 4}, "localFlag": None},
            row("retiring"), row("mine"), row("x", operation="classifier"),
        ])
        report = self.run_cli(old, new)
        expected = [{**shared, "cost": {"input": 3, "output": 4}, "localFlag": None},
                    row("y"), row("mine"), row("x", operation="classifier")]
        self.assertEqual(self.candidate("candidate"), expected)
        self.assertEqual(report["candidate_changes"]["removed"], [
            {"provider": "groq", "type": "chat", "id": "retiring"},
        ])
        self.assertEqual(report["candidate_changes"]["updated"][0]["fields"], [
            {"path": ["cost", "input"], "before": 1, "after": 3},
        ])
        self.assertEqual(report["candidate_records"], 4)
        self.assertEqual(report["conflicts"], [])
        self.assertEqual((self.sources / "groq.json").read_bytes(), self.authoring)

        first = self.root / "candidate/providers"
        repeated = self.run_cli(old, new, sources=first, output="replay")
        self.assertFalse(any(repeated["candidate_changes"].values()))
        self.assertEqual(repeated["identical_additions"], [{"provider": "groq", "type": "chat", "id": "y"}])
        self.assertEqual(repeated["candidate_sha256"], report["candidate_sha256"])
        third = self.release("v3", [{**shared, "cost": {"input": 5, "output": 2}}, row("locally-removed")])
        self.run_cli(new, third, sources=first, output="next")
        self.assertEqual(self.candidate("next"), [
            {**expected[0], "cost": {"input": 5, "output": 4}}, row("mine"), row("x", operation="classifier"),
        ])

    def test_rejects_different_additions_edit_delete_and_overlapping_fields(self):
        old = self.release("v1", [row("x", a=[1], b={"flag": 0}, c=None), row("locally-deleted"),
                                  row("upstream-deleted")])
        new = self.release("v2", [row("collision", first=True), row("locally-deleted", new=True),
                                  row("x", a=[0], b={"flag": None})])
        self.local([row("collision", second=True), row("upstream-deleted", mine=True),
                    row("x", a=[False], b={"flag": False}, c=True)])
        report = self.run_cli(old, new, code=2)
        conflicts = {r["id"]: r["fields"] for r in report["conflicts"]}
        self.assertEqual(set(conflicts), {"collision", "locally-deleted", "upstream-deleted", "x"})
        self.assertEqual(conflicts["collision"][0]["path"], [])
        self.assertNotIn("previous", conflicts["collision"][0])
        self.assertNotIn("local", conflicts["locally-deleted"][0])
        self.assertNotIn("upstream", conflicts["upstream-deleted"][0])
        self.assertEqual(conflicts["x"], [
            {"path": ["a"], "previous": [1], "local": [False], "upstream": [0]},
            {"path": ["b", "flag"], "previous": 0, "local": False, "upstream": None},
            {"path": ["c"], "previous": None, "local": True},
        ])
        self.assertFalse((self.root / "candidate").exists())
        self.assertEqual(report, self.run_cli(old, new, code=2))
        self.assertEqual((self.sources / "groq.json").read_bytes(), self.authoring)

    def test_disjoint_keys_in_new_nested_object_merge_and_arrays_stay_ordered(self):
        old = self.release("v1", [row("x", array=[1, 0])])
        new = self.release("v2", [row("x", compat={"upstream": None}, array=[0, 1])])
        self.local([row("x", compat={"local": None}, array=[1, 0])])
        self.run_cli(old, new)
        self.assertEqual(self.candidate("candidate"),
                         [row("x", array=[0, 1], compat={"local": None, "upstream": None})])

    def test_entire_provider_addition_then_removal(self):
        old = self.release("v1", [])
        new = self.release("v2", [row("new")])
        self.run_cli(old, new)
        self.assertEqual(self.candidate("candidate"), [row("new")])
        report = self.run_cli(new, old, sources=self.root / "candidate/providers", output="removed")
        self.assertEqual(report["candidate_records"], 0)
        self.assertIsNone(report["candidate_sha256"])
        self.assertEqual([p.name for p in (self.root / "removed/providers").iterdir()], ["other.json"])
        self.assertEqual((self.root / "removed/providers/other.json").read_bytes(), self.other)

    def test_rejects_invalid_pins_shards_selection_and_output_paths(self):
        old = self.release("v1", [row("x")])
        new = self.release("v2", [row("x")])
        self.local([row("x")])
        for release in (old, new):
            raw = release[0].read_bytes()
            release[0].write_bytes(raw + b" ")
            self.assertIn("does not match", self.run_cli(old, new, code=1))
            release[0].write_bytes(raw)
        for provider, expected in [("absent", "absent from"), ("../other", "invalid provider")]:
            self.assertIn(expected, self.run_cli(old, new, code=1, provider=provider))
        self.assertIn("outside", self.run_cli(old, new, code=1, output="sources/nested"))
        self.assertFalse((self.sources / "nested").exists())
        (self.sources / "groq.json").write_bytes(b'[{"type":"chat","type":"image"}]')
        self.assertIn("duplicate JSON key", self.run_cli(old, new, code=1))
        self.local([row("x"), row("x")])
        self.assertIn("duplicate model identity", self.run_cli(old, new, code=1))
        self.assertFalse((self.root / "candidate").exists())
        dest = self.root / "candidate"
        dest.mkdir()
        marker = dest / "keep"
        marker.write_bytes(b"keep")
        self.assertIn("already exists", self.run_cli(old, new, code=1))
        self.assertEqual(marker.read_bytes(), b"keep")

    def test_noop_preserves_exact_local_format_and_competing_writers_cannot_overwrite(self):
        old = self.release("v1", [row("x")])
        self.local([row("x")])
        command = self.cmd(old, old)
        writers = [subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        for process in writers:
            process.communicate()
        self.assertEqual(sorted(process.returncode for process in writers), [0, 1])
        self.assertEqual((self.root / "candidate/providers/groq.json").read_bytes(), self.authoring)


if __name__ == "__main__":
    unittest.main()
