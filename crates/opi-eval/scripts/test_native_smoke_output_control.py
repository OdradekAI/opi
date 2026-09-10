#!/usr/bin/env python3
"""Exercise materialization wiring without launching Cargo, clients, or Docker.

Only external commands are replaced. Producer outputs here are explicitly
synthetic stand-ins; actual wire conformance belongs to the ignored Rust probe.
"""

import hashlib
import json
import os
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("native-smoke.sh")
REPO = SCRIPT.resolve().parents[3]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class MaterializeOutputControlTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-smoke-control-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.out = self.root / "out"
        (self.out / "wrappers").mkdir(parents=True)
        self.external = self.root / "external"
        for runner in ("harbor", "pier"):
            home = self.external / runner
            home.mkdir(parents=True)
            (home / "uv.lock").write_text("synthetic lock\n")
        self.agents = self.root / "agents"
        (self.agents / "pi-source/dist/bundle").mkdir(parents=True)
        for name in ("opi", "node", "pi-source/dist/bundle/cli.js",
                     "pi-source/dist/bundle/resource.txt", "pi-source/dist/theme.json",
                     "pi-source/package.json", "pi-source/README.md",
                     "pi-source/CHANGELOG.md"):
            (self.agents / name).write_text(f"synthetic {name}\n")
        for name in ("docs", "examples"):
            (self.agents / "pi-source" / name).mkdir()
            (self.agents / "pi-source" / name / "fixture.md").write_text("synthetic resource\n")
        identities = {
            "opi": {"canonical_executable": str(self.agents / "opi"),
                    "executable_sha256": sha(self.agents / "opi")},
            "pi": {"node_executable": str(self.agents / "node"),
                   "node_sha256": sha(self.agents / "node"),
                   "bundle_path": str(self.agents / "pi-source/dist/bundle/cli.js"),
                   "bundle_sha256": sha(self.agents / "pi-source/dist/bundle/cli.js")},
        }
        for product, identity in identities.items():
            (self.agents / f"{product}-identity.json").write_text(json.dumps(identity))
        self.provider = self.root / "provider"
        self.provider.mkdir()
        (self.provider / "endpoint.txt").write_text("127.0.0.1:32123\n")
        self.lock = self.root / "lock.json"
        self.lock.write_text(json.dumps({
            "subjects": [{"kind": "benchmark-source", "id": family,
                          "task": {"id": "fixture"}} for family in
                         ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")],
            "images": [{"id": name, "reference": "synthetic", "manifest": "sha256:00"}
                       for name in ("tb21-openssl-selfsigned-cert-task",
                                    "tb30-python-base", "deepswe-task")],
        }))
        self.eval = self.root / "opi-eval"
        self.eval.write_text("synthetic companion\n")
        self.calls = []
        self.failure = None

    def command(self, argv, **kwargs):
        self.calls.append(argv)
        if argv == ["sh", "-c", "command -v uv"]:
            return subprocess.CompletedProcess(argv, 0, "/usr/bin/uv\n", "")
        if argv[0] == "cargo":
            self.assertEqual(argv, [
                "cargo", "test", "--locked", "-p", "opi-eval",
                "--test", "native_driver", "native_output_control_client_matrix",
                "--", "--ignored", "--exact",
            ])
            env = kwargs["env"]
            self.assertNotIn("ANTHROPIC_API_KEY", env)
            self.assertNotIn("OPI_EVAL_SCRIPTED_KEY", env)
            self.assertEqual(kwargs["cwd"], str(REPO))
            clients = json.loads(Path(env["OPI_EVAL_NATIVE_CLIENT_MATERIAL"]).read_text())
            self.assertEqual(clients["opi"], str(self.agents / "opi"))
            package = Path(clients["pi_package"])
            self.assertEqual(Path(clients["pi"]).relative_to(package), Path("pi"))
            self.assertEqual((package / "node").read_bytes(), (self.agents / "node").read_bytes())
            for name in ("dist/bundle/cli.js", "dist/bundle/resource.txt", "dist/theme.json",
                         "package.json", "README.md", "CHANGELOG.md",
                         "docs/fixture.md", "examples/fixture.md"):
                self.assertEqual((package / name).read_bytes(),
                                 (self.agents / "pi-source" / name).read_bytes())
            launcher = Path(clients["pi"]).read_text()
            self.assertIn('"$agent_root/node" "$agent_root/dist/bundle/cli.js"', launcher)
            self.assertNotIn(str(self.agents), launcher)
            config = tomllib.loads(Path(clients["experiment"]).read_text())
            self.assertEqual(config["model_controls"]["required"], ["max_output_tokens"])
            self.assertEqual(config["model_controls"]["max_output_tokens"], 4096)
            self.assertEqual(config["benchmark"]["integrity_digest"], "pending")
            self.assertFalse((self.out / "material.json").exists())
            if self.failure == "probe":
                return subprocess.CompletedProcess(argv, 101, "", "synthetic probe failure")
            proof_root = Path(env["OPI_EVAL_OUTPUT_CONTROL_OUT"])
            proof_root.mkdir()
            for product in ("opi", "pi"):
                if self.failure == "missing" and product == "pi":
                    continue
                case = proof_root / product
                case.mkdir()
                (case / "output-control.json").write_text(
                    json.dumps({"synthetic_test_only": product}))
            return subprocess.CompletedProcess(argv, 0, "synthetic probe completed", "")
        if argv[:2] == [str(self.eval), "validate"]:
            material = json.loads((self.out / "material.json").read_text())
            for product in ("opi", "pi"):
                proof = material["agents"][product].get("output_control_evidence")
                self.assertIsNotNone(proof, "validation must follow exact-client proof binding")
                self.assertEqual(proof["sha256"], sha(Path(proof["path"])))
            package = material["agents"]["pi"]["runtime_package"]
            self.assertEqual(package["entrypoint"], "pi")
            self.assertEqual(material["agents"]["pi"]["executable"]["path"],
                             str(Path(package["path"]) / "pi"))
            return subprocess.CompletedProcess(argv, 0, "native_integrity=" + "a" * 64, "")
        self.fail(f"unexpected command (never executed): {argv}")

    def materialize(self):
        source = SCRIPT.read_text(encoding="utf-8").split("<<'MATEOF'\n", 1)[1].split("\nMATEOF", 1)[0]
        argv = ["materialize", str(self.lock), str(self.external), str(self.agents),
                str(self.provider), str(self.out), str(REPO), str(self.eval), "synthetic-network"]
        with patch.object(sys, "argv", argv), patch("subprocess.run", side_effect=self.command), \
                patch.dict(os.environ, {"ANTHROPIC_API_KEY": "synthetic-do-not-forward"}):
            exec(compile(source, str(SCRIPT) + ":MATEOF", "exec"), {"__name__": "__main__"})

    def test_proof_precedes_material_validation_and_config_freeze(self):
        self.materialize()
        config = tomllib.loads((self.out / "terminal-bench-2.1.toml").read_text())
        self.assertEqual(config["benchmark"]["integrity_digest"], "a" * 64)
        self.assertTrue((self.out / "materialize-receipt.json").is_file())

    def test_failed_or_incomplete_probe_never_freezes_material(self):
        for failure in ("probe", "missing"):
            with self.subTest(failure=failure):
                self.failure = failure
                with self.assertRaises(SystemExit):
                    self.materialize()
                self.assertFalse(any(call[:2] == [str(self.eval), "validate"] for call in self.calls))
                self.assertFalse((self.out / "material.json").exists())
                self.assertFalse((self.out / "materialize-receipt.json").exists())
                # Each retry uses fresh output, preserving the prior failed stage.
                self.out = self.root / ("after-" + failure)
                (self.out / "wrappers").mkdir(parents=True)


if __name__ == "__main__":
    unittest.main()
