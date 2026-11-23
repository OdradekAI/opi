import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


SCRIPT = Path(__file__).with_name("pack.py").resolve()
BUILTIN = SCRIPT.parents[2] / "crates/opi-ai/resources/catalog/builtin"


def model(model_id, **fields):
    return {"provider": "groq", "type": "chat", "id": model_id, "api": "openai-completions", **fields}


class CatalogComparisonTest(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.root = Path(scratch.name)
        self.opi = self.write("opi.json", {
            "groq": [
                model("opi-model"),
                model("retiring-upstream"),
                model("shared", maxTokens=300, flags=[False], capabilities={"present_null": None}),
                model("same"),
            ],
        })
        self.builtin_bytes = {p.name: p.read_bytes() for p in BUILTIN.iterdir() if p.is_file()}
        self.opi_bytes = self.opi.read_bytes()

    def tearDown(self):
        self.assertEqual(self.opi.read_bytes(), self.opi_bytes)
        self.assertEqual({p.name: p.read_bytes() for p in BUILTIN.iterdir() if p.is_file()}, self.builtin_bytes)

    def write(self, name, value):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(json.dumps(value).encode("utf-8"))
        return path

    def cli(self, *args):
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--opi-catalog", str(self.opi), "--pi-version", "fixture", *map(str, args)],
            cwd=self.root, capture_output=True,
        )

    def artifact(self, models):
        path = self.write("pi.json", {"groq": models})
        pin = self.write("pin.json", {"revision": "sha256-" + hashlib.sha256(path.read_bytes()).hexdigest()})
        return path, pin

    def snapshot(self, models):
        api = "openai-completions"
        shard = self.write("data/groq.json", {api: {f"{m['type']}:{m['id']}": m for m in models}})
        structure = {"groq": {f"{m['type']}:{m['id']}": api for m in models}}
        raw = json.dumps(structure, sort_keys=True, separators=(",", ":")).encode()
        manifest = self.write("data/.manifest.json", {
            "schemaVersion": 6,
            "structureHash": hashlib.sha256(raw).hexdigest(),
            "files": {"groq.json": hashlib.sha256(shard.read_bytes()).hexdigest()},
        })
        return shard, manifest

    def error(self, result, message, code=1):
        self.assertEqual(result.returncode, code, result)
        self.assertEqual(result.stdout, b"")
        self.assertIn(message.encode(), result.stderr)

    def test_compares_two_references_without_applying_differences(self):
        artifact, pin = self.artifact([model("retiring-upstream"), model("shared", maxTokens=100), model("same")])
        old = self.cli("--pi-artifact", artifact, "--pi-pin", pin)
        self.assertEqual(old.returncode, 0, old.stderr)
        old_report = json.loads(old.stdout)
        self.assertEqual(old_report["reference"]["kind"], "pi-release-artifact")
        self.assertEqual(old_report["reference"]["revision"], json.loads(pin.read_bytes())["revision"])
        self.assertEqual([entry["id"] for entry in old_report["only_opi"]], ["opi-model"])
        self.assertEqual(old_report["only_reference"], [])

        shard, manifest = self.snapshot([
            model("next-upstream"), model("same"),
            model("shared", maxTokens=200, flags=[0], capabilities={"upstream_null": None}),
        ])
        new = self.cli("--pi-provider-data", shard.parent)
        self.assertEqual(new.returncode, 0, new.stderr)
        report = json.loads(new.stdout)
        self.assertEqual(report["reference"], {
            "kind": "pi-provider-snapshot",
            "schema_version": 6,
            "manifest_sha256": hashlib.sha256(manifest.read_bytes()).hexdigest(),
            "structure_sha256": json.loads(manifest.read_bytes())["structureHash"],
            "pi_version": "fixture",
        })
        self.assertEqual(report["counts"], {
            "opi": 4, "reference": 3, "only_opi": 2, "only_reference": 1, "changed": 1, "equal": 1,
        })
        self.assertEqual([entry["id"] for entry in report["only_opi"]], ["opi-model", "retiring-upstream"])
        self.assertEqual([entry["id"] for entry in report["only_reference"]], ["next-upstream"])
        self.assertEqual(report["changed"], [{
            "provider": "groq", "type": "chat", "id": "shared",
            "fields": [
                {"path": ["capabilities", "present_null"], "opi": None},
                {"path": ["capabilities", "upstream_null"], "reference": None},
                {"path": ["flags", 0], "opi": False, "reference": 0},
                {"path": ["maxTokens"], "opi": 300, "reference": 200},
            ],
        }])
        self.assertEqual(new.stdout, self.cli("--pi-provider-data", shard.parent).stdout)

    def test_rejects_wrong_artifact_digest(self):
        artifact, pin = self.artifact([model("same")])
        artifact.write_bytes(artifact.read_bytes() + b"\n")
        self.error(self.cli("--pi-artifact", artifact, "--pi-pin", pin), "does not match sha256-")

    def test_rejects_wrong_shard_digest_and_extra_file(self):
        shard, _ = self.snapshot([model("same")])
        shard.write_bytes(shard.read_bytes() + b"\n")
        self.error(self.cli("--pi-provider-data", shard.parent), "digest does not match")
        (shard.parent / "extra.json").write_bytes(b"{}")
        self.error(self.cli("--pi-provider-data", shard.parent), "extra: ['extra.json']")

    def test_rejects_wrong_snapshot_identity_key(self):
        shard, manifest = self.snapshot([model("same")])
        raw = shard.read_bytes().replace(b"chat:same", b"chat:other")
        shard.write_bytes(raw)
        metadata = json.loads(manifest.read_bytes())
        metadata["files"][shard.name] = hashlib.sha256(raw).hexdigest()
        self.write("data/.manifest.json", metadata)
        self.error(self.cli("--pi-provider-data", shard.parent), "does not match its API or identity key")

    def test_rejects_stale_provider_structure(self):
        shard, manifest = self.snapshot([model("same")])
        metadata = json.loads(manifest.read_bytes())
        metadata["structureHash"] = "0" * 64
        self.write("data/.manifest.json", metadata)
        self.error(self.cli("--pi-provider-data", shard.parent), "structureHash does not match")

    def test_rejects_out_of_range_numbers_with_a_matching_digest(self):
        artifact, pin = self.artifact([model("same", maxTokens="out-of-range")])
        raw = artifact.read_bytes().replace(b'"out-of-range"', b'1e400')
        artifact.write_bytes(raw)
        self.write("pin.json", {"revision": "sha256-" + hashlib.sha256(raw).hexdigest()})
        self.error(self.cli("--pi-artifact", artifact, "--pi-pin", pin), "number out of range: 1e400")

    def test_rejects_duplicate_identity_and_wrong_provider(self):
        for models, diagnostic in [
            ([model("same"), model("same")], "duplicate model identity"),
            ([model("same", provider="anthropic")], "model provider is 'anthropic'"),
        ]:
            with self.subTest(diagnostic=diagnostic):
                artifact, pin = self.artifact(models)
                self.error(self.cli("--pi-artifact", artifact, "--pi-pin", pin), diagnostic)

    def test_requires_digest_pin_and_retires_old_write_invocation(self):
        artifact, _ = self.artifact([model("same")])
        self.error(self.cli("--pi-artifact", artifact), "--pi-pin is required", code=2)
        result = subprocess.run([sys.executable, str(SCRIPT), "--artifact", str(artifact)], capture_output=True)
        self.error(result, "the following arguments are required", code=2)


if __name__ == "__main__":
    unittest.main()
