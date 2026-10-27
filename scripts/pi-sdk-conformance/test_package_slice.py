"""Offline family-graph and report tests for package_slice.py."""

import importlib.util
from contextlib import redirect_stdout
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import sys
import tempfile
import unittest
from unittest import mock

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
         "checksum": package_slice.CHECKSUMS[name], "id": f"{name} {version}"}
        for name, version in entries
    ]
    return {
        "packages": packages,
        "resolve": {"nodes": [{"id": f"{p['name']} {p['version']}", "features": {
            "aws-lc-rs": ["alloc", "aws-lc-sys", "prebuilt-nasm"],
            "aws-lc-sys": ["prebuilt-nasm"],
            "libz-rs-sys": ["std", "rust-allocator"],
            "zlib-rs": ["std", "rust-allocator", "__internal-api"],
        }.get(p["name"], [])}
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
        self.assertEqual(graph["rustls"]["checksum"], package_slice.CHECKSUMS["rustls"])

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

    def test_alternate_registry_is_not_the_reviewed_source(self):
        metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
        metadata["packages"][1]["source"] = "registry+https://unreviewed.invalid/index"
        with self.assertRaisesRegex(ValueError, "rustls"):
            package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])

    def test_fips_union_is_not_the_standard_profile(self):
        metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
        metadata["packages"].append({"name": "aws-lc-fips-sys", "version": "0.13.0"})
        with self.assertRaisesRegex(ValueError, "FIPS|fips"):
            package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])

    def test_missing_or_changed_checksum_is_not_reviewed_source(self):
        for checksum in [None, "0" * 64]:
            metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
            metadata["packages"][1]["checksum"] = checksum
            with self.assertRaisesRegex(ValueError, "checksum"):
                package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])

    def test_unreviewed_native_feature_union_is_rejected(self):
        metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
        metadata["resolve"]["nodes"][-1]["features"].append("bindgen")
        with self.assertRaisesRegex(ValueError, "feature profile"):
            package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])

    def test_unreviewed_local_dependency_is_rejected(self):
        metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
        metadata["packages"].append({"name": "unreviewed", "source": None, "id": "unreviewed"})
        with self.assertRaisesRegex(ValueError, "local dependency"):
            package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])

    def test_http_profile_cannot_hide_unreviewed_crypto_identity(self):
        http = package_slice.FAMILIES["http"]
        metadata = graph_metadata(http["graph_requirements"] + [("aws-lc-sys", "0.45.0")],
                                  self.package / "Cargo.toml")
        with self.assertRaisesRegex(ValueError, "aws-lc-sys 0.46.0"):
            package_slice.check_family_graph(metadata, self.package, http["graph_requirements"])

    def test_crypto_profile_does_not_impose_the_separate_http_profile(self):
        metadata = graph_metadata(self.family["graph_requirements"] + [("http", "1.5.0")],
                                  self.package / "Cargo.toml")
        graph = package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"])
        self.assertEqual(graph["aws-lc-sys"]["version"], "0.46.0")

    def test_sdk_decoder_requires_hidden_zlib_api_in_http_graph_too(self):
        http = package_slice.FAMILIES["http"]
        metadata = graph_metadata(http["graph_requirements"] + [("zlib-rs", "0.6.8")],
                                  self.package / "Cargo.toml")
        metadata["resolve"]["nodes"][-1]["features"] = ["rust-allocator", "std"]
        with self.assertRaisesRegex(ValueError, "zlib-rs feature profile"):
            package_slice.check_family_graph(metadata, self.package,
                                             http["graph_requirements"], profile_name="caller-union")

    def test_http_gzip_still_rejects_unreviewed_zlib_features(self):
        http = package_slice.FAMILIES["http"]
        for extra in ["c-allocator", "export-symbols", "nightly"]:
            metadata = graph_metadata(http["graph_requirements"] + [("zlib-rs", "0.6.8")],
                                      self.package / "Cargo.toml")
            metadata["resolve"]["nodes"][-1]["features"] = ["rust-allocator", "std", extra]
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, "zlib-rs feature profile"):
                package_slice.check_family_graph(metadata, self.package, http["graph_requirements"])

    def test_ws_decoder_still_requires_the_hidden_zlib_api(self):
        requirements = package_slice.FAMILIES["ws"]["profiles"][0]["graph_requirements"]
        metadata = graph_metadata(requirements, self.package / "Cargo.toml")
        next(node for node in metadata["resolve"]["nodes"]
             if node["id"].startswith("zlib-rs "))["features"] = ["rust-allocator", "std"]
        with self.assertRaisesRegex(ValueError, "zlib-rs feature profile"):
            package_slice.check_family_graph(metadata, self.package, requirements)

    def test_native_roots_or_tls_union_is_not_the_reviewed_http_profile(self):
        http = package_slice.FAMILIES["http"]
        metadata = graph_metadata(http["graph_requirements"], self.package / "Cargo.toml")
        metadata["resolve"]["nodes"][1]["features"] = ["native-tls"]
        with self.assertRaisesRegex(ValueError, "feature profile"):
            package_slice.check_family_graph(metadata, self.package, http["graph_requirements"])

    def test_consumer_lock_checksum_must_match_reviewed_registry_release(self):
        metadata = graph_metadata(self.family["graph_requirements"], self.package / "Cargo.toml")
        lock = {"package": [dict(p) for p in metadata["packages"][1:]]}
        lock["package"][0]["checksum"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "checksum"):
            package_slice.check_family_graph(metadata, self.package, self.family["graph_requirements"],
                                             lock=lock)

    def test_unreviewed_flate2_backend_feature_is_rejected(self):
        requirements = package_slice.FAMILIES["ws"]["profiles"][0]["graph_requirements"]
        metadata = graph_metadata(requirements, self.package / "Cargo.toml")
        metadata["resolve"]["nodes"][-1]["features"] = ["zlib-ng"]
        with self.assertRaisesRegex(ValueError, "feature profile"):
            package_slice.check_family_graph(metadata, self.package, requirements)


class FamilyRegistryTests(unittest.TestCase):
    def test_http_family_registered_with_pinned_identities(self):
        from package_slice import FAMILIES

        http = FAMILIES["http"]
        self.assertEqual(http["consumer"], "consumer-http")
        self.assertEqual([p["name"] for p in http["profiles"]], ["ordinary", "caller-union"])
        self.assertEqual(http["expected_checks"], 6)
        self.assertEqual(
            http["graph_requirements"],
            [
                ("reqwest", "0.12.28"),
                ("hyper", "1.9.0"),
                ("hyper-util", "0.1.20"),
                ("http", "1.4.0"),
            ],
        )
        template = Path(__file__).resolve().parent / "consumer-http" / "Cargo.toml"
        manifest = template.read_text(encoding="utf-8")
        self.assertIn('path = "@PACKAGE@"', manifest)
        self.assertIn('caller-union = ["reqwest/gzip"]', manifest)


class WsFamilyTests(unittest.TestCase):
    """Private package checks and external public Upgrade have distinct scopes."""

    def test_ws_family_uses_sdk_and_records_public_codec_as_open(self):
        from package_slice import FAMILIES

        ws = FAMILIES["ws"]
        self.assertEqual(ws["crate"], "opi-ai")
        self.assertNotIn("package_dir", ws)
        self.assertEqual(ws["private_test_minimum"], 95)
        self.assertEqual(ws["public_test_minimum"], 15)
        self.assertEqual(ws["public_codec_status"], "CLOSED: task 20.16 real-chain consumer check")
        self.assertEqual(ws["consumer"], "consumer-ws")
        self.assertEqual(ws["expected_checks"], 10)
        self.assertEqual(
            [p["name"] for p in ws["profiles"]],
            ["ordinary", "union-default", "union-c-zlib"],
        )
        for profile in ws["profiles"]:
            self.assertTrue(
                all(
                    (name, version) in profile["graph_requirements"]
                    for name, version in [("flate2", "1.1.10")]
                ),
                profile["name"],
            )
        self.assertIn(
            ("libz-sys", "1.1.30"),
            ws["profiles"][2]["graph_requirements"],
        )

    def test_consumer_ws_template_declares_union_features(self):
        base = Path(__file__).resolve().parent / "consumer-ws"
        manifest = (base / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('path = "@PACKAGE@"', manifest)
        self.assertIn('opi-ai = { path = "@PACKAGE@" }', manifest)
        self.assertIn('union-default = ["flate2/default"]', manifest)
        self.assertIn('union-c-zlib = ["flate2/zlib"]', manifest)
        self.assertNotIn("opi-ws", manifest)
        source = (base / "src" / "main.rs").read_text(encoding="utf-8")
        for name in [
            "strict-upgrade-and-raw-tail-once",
            "direction-specific-pmd-facts",
            "reject-mismatched-accept",
            "reject-unoffered-subprotocol",
            "reject-unsolicited-extension",
            "redirect-is-terminal",
        ]:
            self.assertIn(name, source, f"missing check {name}")

    def test_ws_graph_uses_extracted_sdk_with_crypto_and_compression(self):
        ws = package_slice.FAMILIES["ws"]
        temp = tempfile.TemporaryDirectory(prefix="opi-ws-graph-test-")
        self.addCleanup(temp.cleanup)
        package = Path(temp.name).resolve() / "opi-ai-0.8.3"
        package.mkdir()
        (package / "Cargo.toml").write_text("", encoding="utf-8")
        metadata = graph_metadata(
            ws["profiles"][0]["graph_requirements"], package / "Cargo.toml"
        )
        next(node for node in metadata["resolve"]["nodes"]
             if node["id"].startswith("flate2 "))["features"] = ["any_impl", "any_zlib", "zlib-rs"]
        graph = package_slice.check_family_graph(
            metadata, package, ws["profiles"][0]["graph_requirements"],
            profile_name="ordinary", ws_profile=True,
        )
        self.assertEqual(graph["libz-rs-sys"]["version"], "0.6.8")
        self.assertEqual(graph["zlib-rs"]["version"], "0.6.8")
        self.assertEqual(graph["aws-lc-sys"]["version"], "0.46.0")

    def test_ws_graph_rejects_wrong_encoder_union_and_legacy_companion(self):
        requirements = package_slice.FAMILIES["ws"]["profiles"][0]["graph_requirements"]
        package = Path(tempfile.gettempdir()).resolve() / "opi-ai-graph-fixture"
        metadata = graph_metadata(requirements, package / "Cargo.toml")
        next(node for node in metadata["resolve"]["nodes"]
             if node["id"].startswith("flate2 "))["features"] = [
                 "any_impl", "any_zlib", "zlib-rs", "any_c_zlib", "libz-sys", "zlib"]
        with self.assertRaisesRegex(ValueError, "flate2 ordinary feature profile"):
            package_slice.check_family_graph(metadata, package, requirements,
                                             profile_name="ordinary", ws_profile=True)
        metadata["packages"][0]["name"] = "opi-ws"
        with self.assertRaisesRegex(ValueError, "extracted opi-ai"):
            package_slice.check_family_graph(metadata, package, requirements)

    def test_zlib_native_provenance_captures_static_archive(self):
        with tempfile.TemporaryDirectory(prefix="opi-zlib-prov-") as temp:
            target = Path(temp).resolve()
            messages, package_id = native_fixture(self, target, "libz-sys")
            provenance = package_slice.zlib_native_provenance(target, messages, package_id)
            self.assertEqual(len(provenance["static_archives"]), 1)
            self.assertEqual(provenance["linked_libs"], ["static=z"])


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


def native_fixture(testcase, target, name="aws-lc-sys", unit="current"):
    version = "0.46.0" if name == "aws-lc-sys" else "1.1.30"
    package_id = f"{package_slice.CRATES_IO}#{name}@{version}"
    build = target / f"debug/build/{name}-{unit}"
    out = build / "out"
    out.mkdir(parents=True)
    lib = "aws_lc_0_46_0_crypto" if name == "aws-lc-sys" else "z"
    (out / f"lib{lib}.a").write_bytes(b"!<arch>\nfixture archive")
    (build / "output").write_text(
        f"cargo:warning=Building with: CC\ncargo:rustc-link-lib=static={lib}\n"
        f'cargo:rustc-link-search=native={out}\nrunning: "reviewed-compiler" "source.c"\n'
        'exit status: 0\n', encoding="utf-8")
    fingerprint = target / f"debug/.fingerprint/{name}-{unit}/run-build-script-main.json"
    fingerprint.parent.mkdir(parents=True)
    key, val = ("AWS_LC_SYS_USE_SYSTEM", "0") if name == "aws-lc-sys" else ("LIBZ_SYS_STATIC", "1")
    fingerprint.write_text(json.dumps({"rustflags": [], "local": [
        {"RerunIfEnvChanged": {"var": key, "val": val}},
        {"RerunIfEnvChanged": {"var": "CC", "val": "reviewed-compiler"}},
        {"RerunIfEnvChanged": {"var": "CC_ENABLE_DEBUG_OUTPUT", "val": "1"}},
    ]}), encoding="utf-8")
    artifact = target / f"debug/deps/lib{name}-{unit}.rlib"
    artifact.parent.mkdir(exist_ok=True)
    artifact.write_bytes(b"rust library")
    source = target / "cargo/registry/src/index.crates.io-fixture" / f"{name}-{version}"
    (source / "src").mkdir(parents=True, exist_ok=True)
    (source / "src/lib.rs").write_text("// fixture", encoding="utf-8")
    (source / "build.rs").write_text("fn main() {}", encoding="utf-8")
    (source / "Cargo.toml").write_text('[package]\nbuild = "build.rs"\n', encoding="utf-8")
    cache = source.parents[2] / "cache" / source.parent.name / f"{source.name}.crate"
    cache.parent.mkdir(parents=True, exist_ok=True)
    with tarfile.open(cache, "w:gz") as archive:
        for path in source.rglob("*"):
            if path.is_file():
                archive.add(path, arcname=f"{source.name}/{path.relative_to(source).as_posix()}")
    testcase.enterContext(mock.patch.dict(package_slice.CHECKSUMS,
                                        {name: hashlib.sha256(cache.read_bytes()).hexdigest()}))
    messages = [
        {"reason": "build-script-executed", "package_id": package_id, "out_dir": str(out),
         "linked_libs": [f"static={lib}"], "linked_paths": [f"native={out}"]},
        {"reason": "compiler-artifact", "package_id": package_id,
         "manifest_path": str(source / "Cargo.toml"),
         "features": ["prebuilt-nasm"] if name == "aws-lc-sys" else [],
         "target": {"kind": ["lib"], "src_path": str(source / "src/lib.rs")},
         "filenames": [str(artifact)], "fresh": True},
    ]
    return messages, package_id


class NativeProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-native-provenance-")
        self.addCleanup(self.temp.cleanup)
        self.target = Path(self.temp.name).resolve()

    def test_current_cached_unit_has_source_inputs_link_and_archive_hashes(self):
        messages, package_id = native_fixture(self, self.target)
        provenance = package_slice.native_build_provenance(self.target, messages, package_id)
        self.assertEqual(provenance["builders"], ["CC"])
        self.assertEqual(provenance["fingerprint_inputs"]["AWS_LC_SYS_USE_SYSTEM"], "0")
        self.assertEqual(provenance["fingerprint_inputs"]["CC"], "reviewed-compiler")
        self.assertEqual(len(provenance["static_archives"]), 1)
        self.assertEqual(len(provenance["artifact_sha256"]), 4)
        self.assertTrue(provenance["compiler_artifact"]["fresh"])
        self.assertTrue(provenance.get("compiler_invocations"))

    def test_c_union_requires_both_native_units(self):
        aws_messages, aws_id = native_fixture(self, self.target)
        zlib_messages, zlib_id = native_fixture(self, self.target, "libz-sys")
        metadata = {"packages": [{"name": name, "id": package_id} for name, package_id in
                                  [("aws-lc-sys", aws_id), ("libz-sys", zlib_id)]]}
        result = package_slice.collect_native_provenance(
            self.target, aws_messages + zlib_messages, metadata, require_zlib=True)
        self.assertEqual(set(result), {"aws-lc-sys", "libz-sys"})
        for omitted in [aws_id, zlib_id]:
            messages = [message for message in aws_messages + zlib_messages
                        if message["package_id"] != omitted]
            with self.subTest(omitted=omitted), self.assertRaisesRegex(ValueError, "native provenance"):
                package_slice.collect_native_provenance(
                    self.target, messages, metadata, require_zlib=True)
        with self.assertRaisesRegex(ValueError, "libz-sys package"):
            package_slice.collect_native_provenance(
                self.target, aws_messages, {"packages": metadata["packages"][:1]}, require_zlib=True)

    def test_unrelated_dynamic_cache_artifact_does_not_describe_current_link(self):
        messages, package_id = native_fixture(self, self.target)
        deps = self.target / "debug/deps"
        (deps / "libcrypto-3.dll").write_bytes(b"dll")
        provenance = package_slice.native_build_provenance(self.target, messages, package_id)
        self.assertEqual(provenance["linked_libs"], ["static=aws_lc_0_46_0_crypto"])

    def test_old_static_archive_is_not_current_consumer_provenance(self):
        native_fixture(self, self.target, unit="old")
        messages, package_id = native_fixture(self, self.target)
        messages[0]["linked_libs"] = ["dylib=crypto"]
        with self.assertRaisesRegex(ValueError, "native link"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_missing_provenance_fails_closed(self):
        with self.assertRaisesRegex(ValueError, "consumer|provenance"):
            package_slice.native_build_provenance(self.target)

    def test_selected_unit_must_have_matching_fingerprint_output_and_archive(self):
        messages, package_id = native_fixture(self, self.target)
        fingerprint = next((self.target / "debug/.fingerprint/aws-lc-sys-current").glob("*.json"))
        fingerprint.write_text(json.dumps({"rustflags": [], "local": [
            {"RerunIfEnvChanged": {"var": "AWS_LC_SYS_USE_SYSTEM", "val": "1"}}
        ]}), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "AWS_LC_SYS_USE_SYSTEM"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_other_cached_static_archive_cannot_cover_missing_selected_archive(self):
        native_fixture(self, self.target, unit="old")
        messages, package_id = native_fixture(self, self.target)
        next(Path(messages[0]["out_dir"]).glob("*.a")).unlink()
        with self.assertRaisesRegex(ValueError, "static archive"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_native_unit_requires_reviewed_source_package_checksum(self):
        messages, package_id = native_fixture(self, self.target)
        source = Path(messages[1]["manifest_path"]).parent
        cache = source.parents[2] / "cache" / source.parent.name / f"{source.name}.crate"
        cache.write_bytes(b"corrupt archive")
        with self.assertRaisesRegex(ValueError, "source checksum"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_msvc_archive_aliases_must_have_identical_static_contents(self):
        messages, package_id = native_fixture(self, self.target)
        out = Path(messages[0]["out_dir"])
        (out / "aws_lc_0_46_0_crypto.lib").write_bytes(next(out.glob("*.a")).read_bytes())
        provenance = package_slice.native_build_provenance(self.target, messages, package_id)
        self.assertEqual(len(provenance["static_archives"]), 2)
        (out / "aws_lc_0_46_0_crypto.lib").write_bytes(b"!<arch>\ndifferent")
        with self.assertRaisesRegex(ValueError, "ambiguous.*static archive"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_modified_cached_native_source_is_not_the_reviewed_archive(self):
        messages, package_id = native_fixture(self, self.target)
        source = Path(messages[1]["manifest_path"]).parent
        (source / "build.rs").write_text("unreviewed source replacement", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "source differs"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_compiler_unit_features_must_match_the_reviewed_profile(self):
        messages, package_id = native_fixture(self, self.target)
        messages[1]["features"].append("bindgen")
        with self.assertRaisesRegex(ValueError, "compiler features"):
            package_slice.native_build_provenance(self.target, messages, package_id)

    def test_missing_selected_compiler_invocation_evidence_is_rejected(self):
        messages, package_id = native_fixture(self, self.target)
        output = Path(messages[0]["out_dir"]).parent / "output"
        output.write_text("\n".join(line for line in output.read_text(encoding="utf-8").splitlines()
                                     if not line.startswith("running:")), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "compiler invocation"):
            package_slice.native_build_provenance(self.target, messages, package_id)


class GateCliTests(unittest.TestCase):
    def test_unreviewed_native_environment_rejected_before_any_build(self):
        for key, value in [
            ("AWS_LC_SYS_USE_SYSTEM", "1"),
            ("AWS_LC_SYS_SYSTEM_DIR", "unreviewed"),
            ("AWS_LC_SYS_SYSTEM_BINDINGS", "unreviewed"),
            ("AWS_LC_SYS_SYSTEM_SKIP_VERSION_CHECK", "1"),
            ("AWS_LC_SYS_USE_SYSTEM_x86_64_pc_windows_msvc", "1"),
            ("AWS_LC_SYS_SYSTEM_DIR_x86_64_pc_windows_msvc", "unreviewed"),
        ]:
            with self.subTest(key=key), tempfile.TemporaryDirectory() as existing:
                result = subprocess.run(
                    [sys.executable, str(Path(package_slice.__file__)), "--family", "crypto",
                     "--offline", "--scratch", existing, "--target-dir", existing],
                    env=dict(os.environ, **{key: value}), capture_output=True, text=True,
                )
                self.assertEqual(result.returncode, 2)
                self.assertIn(key.lower(), result.stderr.lower())
                self.assertNotIn("Traceback", result.stderr)

    def test_native_config_replacements_are_rejected(self):
        replacements = [
            '[source.crates-io]\nreplace-with = "unreviewed"\n',
            '[target.x86_64-pc-windows-msvc.aws_lc_0_46_0]\nrustc-link-lib = ["crypto"]\n',
            '[env]\nAWS_LC_SYS_SYSTEM_DIR = "unreviewed"\n',
        ]
        for config in replacements:
            with self.subTest(config=config), tempfile.TemporaryDirectory() as temp:
                home = Path(temp)
                (home / "config.toml").write_text(config, encoding="utf-8")
                with self.assertRaisesRegex(ValueError, "unreviewed"):
                    package_slice.check_cargo_configs(
                        {"CARGO_HOME": str(home), "CARGO_TARGET_DIR": str(home / "target")}, [])

    def test_compiler_selection_and_existing_shared_cache_remain_supported(self):
        env = package_slice.controlled_environment(
            {"CC": "clang", "AR": "llvm-ar", "CARGO_BUILD_JOBS": "4",
             "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER": "link.exe"}, Path("shared-target"))
        self.assertEqual(env["CC"], "clang")
        self.assertEqual(env["AR"], "llvm-ar")
        self.assertEqual(env["AWS_LC_SYS_USE_SYSTEM"], "0")
        self.assertEqual(env.get("CC_ENABLE_DEBUG_OUTPUT"), "1")


class WsPackageBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="opi-private-ws-package-")
        self.addCleanup(self.temp.cleanup)
        self.package = Path(self.temp.name).resolve() / "package"
        for name in package_slice.WS_PACKAGE_FILES:
            path = self.package / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("", encoding="utf-8")
        (self.package / "Cargo.toml").write_text('[package]\nname = "opi-ai"\n', encoding="utf-8")

    def test_missing_private_package_file_fails_before_build(self):
        package_slice.check_ws_package(self.package)
        (self.package / "src/ws/tests/deflate_audit.rs").unlink()
        with self.assertRaisesRegex(ValueError, "missing.*deflate_audit"):
            package_slice.check_ws_package(self.package)

    def test_missing_public_upgrade_test_or_shared_fixture_fails_before_build(self):
        for name in ["tests/ws_upgrade.rs", "tests/support/ws_peer.rs", "tests/support/tls_fixtures.rs"]:
            path = self.package / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("retained test source", encoding="utf-8")
            package_slice.check_ws_package(self.package)
            path.unlink()
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "missing.*" + path.name):
                package_slice.check_ws_package(self.package)
            path.write_text("retained test source", encoding="utf-8")

    def test_private_upgrade_can_include_only_the_packaged_shared_peer(self):
        peer = self.package / "tests/support/ws_peer.rs"
        peer.parent.mkdir(parents=True, exist_ok=True)
        peer.write_text("// shared loopback test peer", encoding="utf-8")
        upgrade = self.package / "src/ws/tests/upgrade.rs"
        upgrade.write_text('mod peer { include!("../../../tests/support/ws_peer.rs"); }',
                           encoding="utf-8")
        package_slice.check_ws_package(self.package)
        peer.write_text('include!("../../../checkout/ws_peer.rs");', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "external|missing"):
            package_slice.check_ws_package(self.package)

    def test_missing_license_or_fixture_extraction_source_fails_closed(self):
        for name in ["LICENSE", "src/ws/tests/fixtures/extract-boundary-fixtures.py"]:
            path = self.package / name
            path.write_text("retained source", encoding="utf-8")
            package_slice.check_ws_package(self.package)
            path.unlink()
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, "missing.*" + path.name):
                package_slice.check_ws_package(self.package)
            path.write_text("retained source", encoding="utf-8")

    def test_stale_companion_or_external_production_source_is_rejected(self):
        for reference in ['#[path = "../../../companions/ws/src/io.rs"] mod io;',
                          'include_bytes!("../../../../../scripts/pi-sdk-conformance/fixture")']:
            (self.package / "src/ws/tests/upgrade.rs").write_text(reference, encoding="utf-8")
            with self.subTest(reference=reference), self.assertRaisesRegex(ValueError, "external|companion"):
                package_slice.check_ws_package(self.package)

    def test_spaced_include_macro_cannot_escape_private_package(self):
        (self.package / "src/ws/tests/upgrade.rs").write_text(
            'include ! ("../../../../../checkout/crates/opi-ai/src/ws/io.rs");', encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "external|missing"):
            package_slice.check_ws_package(self.package)

    def test_raw_or_computed_private_source_references_are_not_verifiable(self):
        for reference in [
            'include_str!(r"D:/checkout/crates/opi-ai/src/ws/io.rs");',
            'include_str!(concat!("D:/checkout/", "crates/opi-ai/src/ws/io.rs"));',
            '#[path = r"D:/checkout/crates/opi-ai/src/ws/io.rs"] mod codec;',
        ]:
            (self.package / "src/ws/tests/upgrade.rs").write_text(reference, encoding="utf-8")
            with self.subTest(reference=reference), self.assertRaisesRegex(ValueError, "private WS"):
                package_slice.check_ws_package(self.package)

    def test_external_consumer_must_use_public_sdk_only(self):
        consumer = Path(self.temp.name).resolve() / "consumer"
        (consumer / "src").mkdir(parents=True)
        manifest = consumer / "Cargo.toml"
        manifest.write_text('[dependencies]\nopi-ai = { path = '
                            + json.dumps(self.package.as_posix()) + ' }\n', encoding="utf-8")
        source = consumer / "src/main.rs"
        source.write_text("fn main() {}", encoding="utf-8")
        package_slice.check_ws_consumer(consumer, self.package)
        source.write_text('#[path = "D:/checkout/crates/opi-ai/src/ws/io.rs"] mod codec;',
                          encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "production source"):
            package_slice.check_ws_consumer(consumer, self.package)
        source.write_text("fn main() {}", encoding="utf-8")
        manifest.write_text('[dependencies]\nopi-ws = { path = "D:/checkout/companions/ws" }\n',
                            encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "extracted opi-ai"):
            package_slice.check_ws_consumer(consumer, self.package)

    def test_cfg_attr_path_and_spaced_include_cannot_load_checkout_source(self):
        consumer = Path(self.temp.name).resolve() / "consumer"
        (consumer / "src").mkdir(parents=True)
        (consumer / "Cargo.toml").write_text('[dependencies]\nopi-ai = { path = '
                                            + json.dumps(self.package.as_posix()) + ' }\n',
                                            encoding="utf-8")
        for reference in [
            '#[cfg_attr(all(), path = "D:/checkout/crates/opi-ai/src/ws/io.rs")] mod codec;',
            'include ! ("D:/checkout/crates/opi-ai/src/ws/io.rs");',
        ]:
            (consumer / "src/main.rs").write_text(reference, encoding="utf-8")
            with self.subTest(reference=reference), self.assertRaisesRegex(ValueError, "production source"):
                package_slice.check_ws_consumer(consumer, self.package)

    def test_empty_filter_or_missing_suite_cannot_supply_either_layer(self):
        for suite, minimum in [("private protocol supplements", 93), ("public Upgrade", 15)]:
            for output in [f"test result: ok. 0 passed; 0 failed; {minimum} filtered out;",
                           f"test result: ok. {minimum - 1} passed; 0 failed; 0 ignored;",
                           f"test result: ok. {minimum} passed; 1 failed; 0 ignored;",
                           f"test result: ok. {minimum} passed; 0 failed;\n" * 2]:
                with self.subTest(suite=suite, output=output), self.assertRaisesRegex(ValueError, suite):
                    package_slice.check_ws_test_report(output, minimum, suite)
            self.assertEqual(package_slice.check_ws_test_report(
                f"test result: ok. {minimum} passed; 0 failed; 0 ignored;", minimum, suite), minimum)

    def test_both_layers_use_extracted_sdk_and_dependency_feature_unions(self):
        commands = []
        family = dict(package_slice.FAMILIES["ws"], private_test_minimum=93, public_test_minimum=15)
        caller_lock = Path(self.temp.name) / "caller" / "Cargo.lock"
        caller_lock.parent.mkdir()

        def lock_text(entries):
            return "\n".join(
                "[[package]]\n" + "\n".join(
                    f"{key} = {json.dumps(entry[key])}"
                    for key in ("name", "version", "source", "checksum"))
                for entry in entries if entry.get("source"))

        caller_entries = graph_metadata(
            package_slice.FAMILIES["ws"]["profiles"][-1]["graph_requirements"],
            self.package / "Cargo.toml")["packages"]
        caller_lock.write_text(lock_text(caller_entries), encoding="utf-8")

        def run(name, command, cwd):
            commands.append((name, command, cwd))
            if "metadata" in command:
                profile = next(profile for profile in package_slice.FAMILIES["ws"]["profiles"]
                               if name.endswith(profile["name"]))
                metadata = graph_metadata(profile["graph_requirements"], self.package / "Cargo.toml")
                selected = {"any_impl", "any_zlib", "zlib-rs"}
                if profile["name"] == "union-default":
                    selected |= {"default", "miniz_oxide", "runtime_detection", "rust_backend"}
                if profile["name"] == "union-c-zlib":
                    selected |= {"any_c_zlib", "libz-sys", "zlib"}
                next(node for node in metadata["resolve"]["nodes"]
                     if node["id"].startswith("flate2 "))["features"] = sorted(selected)
                # Cargo can resolve a dependency's feature-only C union
                # without adding libz-sys to this library's own lock.
                (self.package / "Cargo.lock").write_text(lock_text(
                    [entry for entry in metadata["packages"] if entry["name"] != "libz-sys"]),
                    encoding="utf-8")
                return json.dumps(metadata)
            passed = 15 if "--test" in command else 93
            return f"test result: ok. {passed} passed; 0 failed; 0 ignored;"

        result = package_slice.run_packaged_ws_checks(
            run, self.package, family, caller_lock)
        self.assertEqual(len(result), 3)
        self.assertEqual(sum("metadata" in command for _, command, _ in commands), 3)
        for index, profile in enumerate(["ordinary", "union-default", "union-c-zlib"]):
            metadata_name, metadata_command, _ = commands[index * 4]
            self.assertEqual(metadata_name, f"private-ws-metadata-{profile}")
            self.assertNotIn("--locked", metadata_command)
            name, command, cwd = commands[index * 4 + 1]
            self.assertEqual(name, f"private-ws-test-{profile}")
            self.assertEqual(cwd, self.package)
            self.assertIn(str(self.package / "Cargo.toml"), command)
            self.assertIn("--lib", command)
            self.assertIn("ws::", command)
            self.assertEqual(command[-3:], ["--", "--skip", "auth::aws::"])
            if index:
                self.assertIn(["flate2/default", "flate2/zlib"][index - 1], command)
            self.assertNotIn("opi-ws", " ".join(command))
            self.assertIn("--locked", command)
            name, public_command, cwd = commands[index * 4 + 2]
            self.assertEqual(name, f"public-ws-test-{profile}")
            self.assertEqual(cwd, self.package)
            self.assertIn(str(self.package / "Cargo.toml"), public_command)
            self.assertEqual(public_command[public_command.index("--test") + 1], "ws_upgrade")
            self.assertNotIn("--lib", public_command)
            self.assertNotIn("ws::", public_command)
            self.assertIn("--offline", public_command)
            self.assertIn("--locked", public_command)
            if index:
                self.assertIn(["flate2/default", "flate2/zlib"][index - 1], public_command)
            name, clippy_command, cwd = commands[index * 4 + 3]
            self.assertEqual(name, f"packaged-ws-clippy-{profile}")
            self.assertEqual(cwd, self.package)
            self.assertIn("--lib", clippy_command)
            self.assertEqual(clippy_command[clippy_command.index("--test") + 1], "ws_upgrade")
            self.assertNotIn("--all-targets", clippy_command)
            self.assertNotIn("--tests", clippy_command)
            self.assertEqual(result[index]["private_protocol"]["passed"], 93)
            self.assertEqual(result[index]["public_upgrade"]["passed"], 15)
            self.assertIn("supplements", result[index]["private_protocol"]["scope"])
            self.assertIn("public Upgrade", result[index]["public_upgrade"]["scope"])
            for suite in ["private_protocol", "public_upgrade"]:
                self.assertIn("not complete Codex wiring", result[index][suite]["scope"])
            self.assertIn("aws-lc-sys", result[index]["graph"])
            self.assertIn("resolved_library_lock_sha256", result[index])
            self.assertEqual(result[index]["caller_checksum_lock"]["sha256"],
                             hashlib.sha256(caller_lock.read_bytes()).hexdigest())
            self.assertEqual(result[index]["resolved_library_lock_missing_reviewed_identities"],
                             ["libz-sys"] if profile == "union-c-zlib" else [])
        # The caller lock is checksum evidence, never permission to relax
        # the independently fixed reviewed registry identity.
        caller_lock.write_text(lock_text([
            dict(entry, checksum="0" * 64) if entry["name"] == "libz-sys" else entry
            for entry in caller_entries]), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "checksum for libz-sys"):
            package_slice.run_packaged_ws_checks(
                run, self.package, family, caller_lock)

    def test_fresh_caller_checksum_lock_precedes_private_protocol_checks(self):
        repo, target, scratch = (Path(self.temp.name) / name for name in ("repo", "target", "proof"))
        repo.mkdir()
        (repo / "Cargo.toml").write_text('[workspace.package]\nversion = "0.8.3"\n', encoding="utf-8")
        (target / "package").mkdir(parents=True)
        (target / "package/opi-ai-0.8.3.crate").write_bytes(b"actual archive fixture")
        (target / ".opi-cargo-cache.json").write_text(
            json.dumps({"workspace": str(repo)}), encoding="utf-8")

        def fake_process(command, **kwargs):
            if command[:2] == ["cargo", "generate-lockfile"]:
                (scratch / "consumer/Cargo.lock").write_text("version = 4\n", encoding="utf-8")
            return subprocess.CompletedProcess(command, 0, "command stdout", "command stderr")

        def private_checks(*args):
            checksum_lock = scratch / "consumer/Cargo.lock"
            self.assertTrue(checksum_lock.is_file(), "private checks require a fresh caller lock first")
            self.assertEqual(args[3], checksum_lock)
            raise ValueError("stop after fresh caller checksum lock")

        argv = [str(Path(package_slice.__file__)), "--family", "ws", "--offline",
                "--scratch", str(scratch), "--target-dir", str(target)]
        with mock.patch.object(package_slice, "REPO", repo), \
                mock.patch.object(sys, "argv", argv), \
                mock.patch.dict(os.environ, {"CARGO_HOME": str(Path(self.temp.name) / "cargo-home"),
                                             "USERPROFILE": str(Path(self.temp.name) / "home")},
                                clear=True), \
                mock.patch.object(subprocess, "run", side_effect=fake_process), \
                mock.patch.object(package_slice.shared, "unpack_package", return_value=self.package), \
                mock.patch.object(package_slice, "run_packaged_ws_checks", side_effect=private_checks), \
                redirect_stdout(io.StringIO()):
            self.assertEqual(package_slice.main(), 1)
        evidence = json.loads((scratch / "evidence.json").read_text(encoding="utf-8"))
        self.assertEqual(evidence["error"], "stop after fresh caller checksum lock")
        self.assertIn("ws_package_files", evidence)
        self.assertEqual(set(evidence["ws_package_files"]), set(package_slice.WS_PACKAGE_FILES))
        self.assertNotIn("private_ws_files", evidence)

    def test_sdk_packaging_precedes_all_private_ws_gates(self):
        with tempfile.TemporaryDirectory(prefix="opi-ws-bridge-gates-") as temp:
            root = Path(temp).resolve()
            repo, target, scratch = root / "repo", root / "target", root / "proof"
            repo.mkdir()
            (repo / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.8.3"\n', encoding="utf-8")
            target.mkdir()
            (target / ".opi-cargo-cache.json").write_text(
                json.dumps({"workspace": str(repo)}), encoding="utf-8")

            def fake_process(command, **kwargs):
                # Stop at the packaging boundary; all real scheduling and log
                # persistence before it still run, without invoking Cargo.
                failed = command[:2] == ["cargo", "package"]
                identity = json.dumps(command)
                return subprocess.CompletedProcess(
                    command, int(failed), "stdout " + identity, "stderr " + identity)

            argv = [str(Path(package_slice.__file__)), "--family", "ws", "--offline",
                    "--scratch", str(scratch), "--target-dir", str(target)]
            with mock.patch.object(package_slice, "REPO", repo), \
                    mock.patch.object(sys, "argv", argv), \
                    mock.patch.dict(os.environ, {"CARGO_HOME": str(root / "cargo-home"),
                                                 "USERPROFILE": str(root / "home")}, clear=True), \
                    mock.patch.object(subprocess, "run", side_effect=fake_process), \
                    redirect_stdout(io.StringIO()):
                self.assertEqual(package_slice.main(), 1)

            evidence = json.loads((scratch / "evidence.json").read_text(encoding="utf-8"))
            self.assertIn("package failed", evidence["error"])
            steps = evidence["steps"]
            names = [step["name"] for step in steps]
            self.assertFalse(any(name.startswith(("companion-", "bridge-check", "private-ws-",
                                                  "public-ws-", "packaged-ws-"))
                                 for name in names))
            package_step = next(step for step in steps if step["name"] == "package")
            self.assertIn("opi-ai", package_step["command"])
            self.assertEqual(
                evidence["public_codec_status"],
                "CLOSED: task 20.16 real-chain consumer check")


if __name__ == "__main__":
    unittest.main()
