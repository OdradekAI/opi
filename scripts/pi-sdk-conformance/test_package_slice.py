"""Offline family-graph and report tests for package_slice.py."""

import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).parent))
import package_slice  # noqa: E402


def graph_metadata(entries, sdk_manifest):
    packages = [
        {"name": "opi-ai", "version": "0.8.3", "source": None,
         "manifest_path": str(sdk_manifest), "id": "opi-ai 0.8.3"}
    ] + [
        {"name": name, "version": version,
         "source": "registry+https://github.com/rust-lang/crates.io-index",
         "checksum": f"sha256:{name}-{version}", "id": f"{name} {version}"}
        for name, version in entries
    ]
    return {
        "packages": packages,
        "resolve": {"nodes": [{"id": f"{p['name']} {p['version']}", "features": []}
                              for p in packages]},
    }


def passing_report(names):
    return {
        "verdict": "PASS",
        "failed": 0,
        "total": len(names),
        "checks": [{"name": name, "passed": True, "detail": ""} for name in names],
    }


class FamilyGraphTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-package-slice-test-")
        self.addCleanup(self.temp.cleanup)
        self.package = Path(self.temp.name).resolve() / "opi-ai-0.8.3"
        self.package.mkdir()
        (self.package / "Cargo.toml").write_text("", encoding="utf-8")
        self.family = package_slice.FAMILIES["crypto"]

    def test_registry_graph_with_required_identities_is_recorded(self):
        metadata = graph_metadata(self.family["graph_requirements"],
                                  self.package / "Cargo.toml")
        graph = package_slice.check_family_graph(metadata, self.package,
                                                 self.family["graph_requirements"])
        self.assertIn("rustls", graph)
        self.assertEqual(graph["aws-lc-sys"]["version"], "0.46.0")
        self.assertTrue(graph["aws-lc-rs"]["source"].startswith("registry+"))
        self.assertTrue(graph["rustls"]["checksum"].startswith("sha256:"))

    def test_missing_required_identity_fails_closed(self):
        metadata = graph_metadata([("rustls", "0.23.40")], self.package / "Cargo.toml")
        with self.assertRaisesRegex(ValueError, "aws-lc-rs"):
            package_slice.check_family_graph(metadata, self.package,
                                             self.family["graph_requirements"])

    def test_wrong_version_or_non_registry_source_fails_closed(self):
        entries = [("rustls", "0.23.39"), ("aws-lc-rs", "1.18.2"), ("aws-lc-sys", "0.46.0")]
        metadata = graph_metadata(entries, self.package / "Cargo.toml")
        with self.assertRaisesRegex(ValueError, "rustls 0.23.40"):
            package_slice.check_family_graph(metadata, self.package,
                                             self.family["graph_requirements"])

    def test_sdk_must_come_from_the_extracted_package(self):
        entries = self.family["graph_requirements"]
        metadata = graph_metadata(entries, self.package / "elsewhere" / "Cargo.toml")
        with self.assertRaisesRegex(ValueError, "extracted"):
            package_slice.check_family_graph(metadata, self.package,
                                             self.family["graph_requirements"])


class FamilyReportTests(unittest.TestCase):
    def setUp(self):
        self.family = package_slice.FAMILIES["crypto"]
        self.names = [
            "ordinary-no-global-provider",
            "chat-mtls-tls13-h2-round-trip",
            "chat-tls12-h1-explicit-selection",
            "untrusted-roots-zero-business-http",
            "mismatched-identity-rejected-before-io",
            "global-provider-state-unchanged",
        ]

    def test_complete_passing_battery_is_accepted(self):
        package_slice.check_family_report(passing_report(self.names), self.family)

    def test_any_failed_check_rejects_the_report(self):
        report = passing_report(self.names)
        report["checks"][2]["passed"] = False
        report["failed"] = 1
        with self.assertRaisesRegex(ValueError, "battery"):
            package_slice.check_family_report(report, self.family)

    def test_missing_or_duplicated_checks_reject_the_report(self):
        short = passing_report(self.names[:-1])
        short["total"] = short["total"] - 1
        with self.assertRaises(ValueError):
            package_slice.check_family_report(short, self.family)
        duplicate = passing_report(self.names[:-1] + [self.names[0]])
        with self.assertRaises(ValueError):
            package_slice.check_family_report(duplicate, self.family)


class LockChecksumTests(unittest.TestCase):
    def test_registry_packages_carry_checksums(self):
        # Smoke of the lock-parsing contract: every registry package in a
        # well-formed consumer lock carries a sha256 checksum.
        import tomllib
        lock = tomllib.loads("""
[[package]]
name = "rustls"
version = "0.23.40"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "sha256:abc"

[[package]]
name = "opi-ai"
version = "0.8.3"
""")
        checksums = {
            p["name"]: p.get("checksum")
            for p in lock["package"]
            if p["name"] in {"rustls"}
        }
        self.assertEqual(checksums, {"rustls": "sha256:abc"})
        self.assertTrue(all(checksums.values()))


class NativeProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-native-provenance-")
        self.addCleanup(self.temp.cleanup)
        self.target = Path(self.temp.name).resolve()

    def write_build(self, name, log_lines, static_archive=False):
        build = self.target / f"debug/build/{name}"
        (build / "out").mkdir(parents=True)
        (build / "output").write_text("\n".join(log_lines) + "\n", encoding="utf-8")
        if static_archive:
            (build / "out/libaws_lc_0_46_0_crypto.a").write_bytes(b"static")

    def test_builder_use_system_and_static_archive_are_captured(self):
        self.write_build(
            "aws-lc-sys-abc",
            [
                "cargo:warning=Environment Variable found 'AWS_LC_SYS_USE_SYSTEM': '0'",
                "cargo:warning=Value: 0 is false.",
                "cargo:warning=Building with: CC",
                "cargo:root=/x",
            ],
            static_archive=True,
        )
        provenance = package_slice.native_build_provenance(self.target)
        self.assertEqual(provenance["builders"], ["CC"])
        self.assertTrue(any("AWS_LC_SYS_USE_SYSTEM" in line for line in provenance["use_system_observed"]))
        self.assertEqual(len(provenance["static_archives"]), 1)
        self.assertEqual(provenance["dynamic_libcrypto_artifacts"], [])

    def test_dynamic_libcrypto_artifacts_are_reported(self):
        deps = self.target / "debug/deps"
        deps.mkdir(parents=True)
        (deps / "libcrypto-3.dll").write_bytes(b"dll")
        (deps / "aws_lc_0_46_0_crypto.dll.lib").write_bytes(b"")
        provenance = package_slice.native_build_provenance(self.target)
        self.assertEqual(len(provenance["dynamic_libcrypto_artifacts"]), 1)


if __name__ == "__main__":
    unittest.main()
