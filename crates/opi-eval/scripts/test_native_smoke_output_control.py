#!/usr/bin/env python3
"""Exercise materialization wiring without launching Cargo, clients, or Docker.

Only external commands are replaced. Producer outputs here are explicitly
synthetic stand-ins; actual wire conformance belongs to the ignored Rust probe.
The fixture models the current shared preparation seam: it stages real
synthetic executable bytes (including uv) inside fresh fixture roots and mocks
`opi-eval prepare` with explicitly synthetic per-family material/6 outputs, so
no installed uv or any other host tool is required.
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
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


class MaterializeOutputControlTest(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-smoke-control-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.out = self.out_for("out")
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
        # Actual synthetic executable uv bytes live in the fixture root; the
        # producer's `command -v uv` resolution and byte transport then work
        # without any installed uv.
        self.tools = self.root / "tools"
        self.tools.mkdir()
        (self.tools / "uv").write_text("#!/bin/sh\nsynthetic fixture uv\n")
        (self.tools / "uv").chmod(0o755)
        self.provider = self.root / "provider"
        self.provider.mkdir()
        (self.provider / "endpoint.txt").write_text("127.0.0.1:32123\n")
        self.lock = self.root / "lock.json"
        self.lock.write_text(json.dumps({
            "subjects": [{"kind": "benchmark-source", "id": family,
                          "commit": "a" * 40, "tasks_tree": "b" * 40,
                          "task": {"id": "fixture", "tree": "c" * 40}} for family in
                         ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")],
            "images": [{"id": name, "reference": "synthetic", "manifest": "sha256:00"}
                       for name in ("tb21-openssl-selfsigned-cert-task",
                                    "tb30-python-base", "deepswe-task")],
        }))
        for clone in ("terminal-bench-2-1", "terminal-bench", "deep-swe"):
            task = self.external / clone / "tasks" / "fixture"
            task.mkdir(parents=True)
            (task / "task.toml").write_text('[task]\nname = "fixture"\n')
            (task / "instruction.md").write_text("synthetic instruction\n")
        self.eval = self.root / "opi-eval"
        self.eval.write_text("synthetic companion\n")
        self.calls = []
        self.failure = None

    def out_for(self, name):
        out = self.root / name
        (out / "wrappers").mkdir(parents=True)
        return out

    def command(self, argv, **kwargs):
        self.calls.append(argv)
        if argv == ["sh", "-c", "command -v uv"]:
            return subprocess.CompletedProcess(argv, 0, f"{self.tools / 'uv'}\n", "")
        if argv[0] == "cargo":
            self.assertEqual(argv, [
                "cargo", "test", "--locked", "-p", "opi-eval",
                "--test", "native_driver", "native_output_control_client_matrix",
                "--", "--ignored", "--exact",
            ])
            env = kwargs["env"]
            self.assertNotIn("ANTHROPIC_API_KEY", env)
            self.assertNotIn("OPI_EVAL_SCRIPTED_KEY", env)
            self.assertEqual(Path(kwargs["cwd"]), REPO)
            clients = json.loads(Path(env["OPI_EVAL_NATIVE_CLIENT_MATERIAL"]).read_text())
            cache_agents = self.out / "cache" / "agents"
            self.assertEqual(clients["opi"], str(cache_agents / "opi"))
            self.assertEqual(clients["pi"], str(cache_agents / "pi" / "pi"))
            self.assertEqual(clients["pi_package"], str(cache_agents / "pi"))
            package = Path(clients["pi_package"])
            self.assertEqual(Path(clients["pi"]).relative_to(package), Path("pi"))
            # Cached Opi/pi executable and package bytes are transported
            # unchanged from the staged synthetic sources.
            self.assertEqual((package / "node").read_bytes(), (self.agents / "node").read_bytes())
            self.assertEqual(
                (Path(clients["opi"])).read_bytes(), (self.agents / "opi").read_bytes())
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
        if argv[:2] == [str(self.eval), "prepare"]:
            # The shared local preparation is mocked: this fixture never
            # transforms real preparation inputs, it only stages explicitly
            # synthetic per-family material/6 bytes in the requested output.
            self.assertEqual(argv[2:6], ["--config", argv[3], "--out", argv[5]])
            config = Path(argv[3])
            prepared = Path(argv[5])
            prepared.mkdir(parents=True, exist_ok=True)
            cache_agents = self.out / "cache" / "agents"
            manifest = {
                "schema": "opi-eval-native-material/7",
                "origin": "local",
                "static_lock": {"sha256": sha(self.lock)},
                "capture_helper": {
                    "executable": {"path": str(cache_agents / "capture-helper"),
                                   "sha256": "synthetic"},
                    "target": "x86_64-unknown-linux-musl",
                    "protocol": "opi-eval-native-capture-helper/1",
                    "cargo_lock_sha256": "0" * 64,
                },
                "agents": {
                    "opi": {"executable": {"path": str(cache_agents / "opi"),
                                           "sha256": sha(self.agents / "opi")}},
                    "pi": {"executable": {"path": str(cache_agents / "pi" / "pi"),
                                          "sha256": "synthetic"},
                           "runtime_package": {"path": str(cache_agents / "pi"),
                                               "entrypoint": "pi"}},
                },
                "synthetic_fixture_only": config.name,
            }
            (prepared / "material.json").write_text(
                json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            return subprocess.CompletedProcess(argv, 0, "", "")
        if argv[:2] == [str(self.eval), "validate"]:
            self.assertEqual(argv[2:4], ["--config", argv[3]])
            self.assertEqual(argv[4], "--native-material")
            material_path = Path(argv[5])
            self.assertEqual(material_path.parent.parent, self.out)
            self.assertTrue(
                material_path.parent.name.startswith("prepared-"),
                f"validate must consume the actual prepared material: {material_path}")
            adapter = material_path.parent.name.removeprefix("prepared-")
            self.assertEqual(material_path, self.out / f"prepared-{adapter}" / "material.json")
            material = json.loads(material_path.read_text())
            for product in ("opi", "pi"):
                proof = material["agents"][product].get("output_control_evidence")
                self.assertIsNotNone(proof, "validation must follow exact-client proof binding")
                self.assertEqual(proof["sha256"], sha(Path(proof["path"])))
            package = material["agents"]["pi"]["runtime_package"]
            self.assertEqual(package["entrypoint"], "pi")
            self.assertEqual(material["agents"]["pi"]["executable"]["path"],
                             str(Path(package["path"]) / "pi"))
            # Validation still precedes the config freeze.
            config = tomllib.loads(Path(argv[3]).read_text())
            self.assertEqual(config["benchmark"]["integrity_digest"], "pending")
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
        for adapter in ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"):
            config = tomllib.loads((self.out / f"{adapter}.toml").read_text())
            self.assertEqual(config["benchmark"]["integrity_digest"], "a" * 64)
        self.assertTrue((self.out / "materialize-receipt.json").is_file())
        self.assertTrue((self.out / "material-routing.json").is_file())

    def test_failed_or_incomplete_probe_never_freezes_material(self):
        for failure in ("probe", "missing"):
            with self.subTest(failure=failure):
                self.failure = failure
                with self.assertRaises(SystemExit):
                    self.materialize()
                self.assertFalse(any(
                    call[:2] == [str(self.eval), "validate"] for call in self.calls))
                self.assertFalse((self.out / "materialize-receipt.json").exists())
                self.assertFalse((self.out / "material-routing.json").exists())
                for adapter in ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1"):
                    config = self.out / f"{adapter}.toml"
                    if config.is_file():
                        self.assertEqual(
                            tomllib.loads(config.read_text())["benchmark"]["integrity_digest"],
                            "pending")
                # Each retry uses fresh output, preserving the prior failed stage.
                self.out = self.out_for("after-" + failure)


if __name__ == "__main__":
    unittest.main()
