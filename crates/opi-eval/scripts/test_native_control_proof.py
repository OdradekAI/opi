import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("native-control-proof.py")


class ControlProofTest(unittest.TestCase):
    def test_existing_root_refuses_before_client_or_material_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = root / "config.toml"
            material = root / "material.json"
            config.write_text('[[subjects]]\nproduct = "pi"\n')
            original = json.dumps({"agents": {}}).encode()
            material.write_bytes(original)
            output = subprocess.run([sys.executable, str(SCRIPT), "--companion", str(root / "absent"),
                                     "--config", str(config), "--material", str(material), "--out", str(root)],
                                    capture_output=True, text=True)
            self.assertNotEqual(output.returncode, 0)
            self.assertIn("fresh", output.stderr)
            self.assertEqual(material.read_bytes(), original)

    def test_package_binding_rejects_executable_drift_and_escaping_entrypoint(self):
        spec = importlib.util.spec_from_file_location("native_control_proof", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "pi"
            binary.write_bytes(b"client")
            agent = {"executable": module.pin(binary)}
            module.verify_agent(agent)
            agent["executable"]["sha256"] = "0" * 64
            with self.assertRaisesRegex(ValueError, "digest"):
                module.verify_agent(agent)
            agent["executable"] = module.pin(binary)
            agent["runtime_package"] = {"path": str(root), "entrypoint": "../pi", "manifest_sha256": "0" * 64}
            with self.assertRaisesRegex(ValueError, "entrypoint"):
                module.verify_agent(agent)


if __name__ == "__main__":
    unittest.main()
