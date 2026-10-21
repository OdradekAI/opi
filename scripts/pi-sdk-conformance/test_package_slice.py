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

    def test_http_gzip_does_not_require_the_ws_hidden_zlib_api(self):
        http = package_slice.FAMILIES["http"]
        metadata = graph_metadata(http["graph_requirements"] + [("zlib-rs", "0.6.8")],
                                  self.package / "Cargo.toml")
        metadata["resolve"]["nodes"][-1]["features"] = ["rust-allocator", "std"]
        graph = package_slice.check_family_graph(metadata, self.package,
                                                 http["graph_requirements"], profile_name="caller-union")
        self.assertEqual(graph["zlib-rs"]["features"], ["rust-allocator", "std"])

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
        metadata["packages"][0]["name"] = "opi-ws"
        metadata["resolve"]["nodes"][2]["features"] = ["rust-allocator", "std"]
        with self.assertRaisesRegex(ValueError, "zlib-rs feature profile"):
            package_slice.check_family_graph(metadata, self.package, requirements, "opi-ws")

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
    """The ws family packages the standalone companion, not the SDK."""

    def test_ws_family_registered_with_companion_shape(self):
        from package_slice import FAMILIES

        ws = FAMILIES["ws"]
        self.assertEqual(ws["kind"], "companion")
        self.assertEqual(ws["crate"], "opi-ws")
        self.assertEqual(ws["package_dir"], "companions/ws")
        self.assertEqual(ws["consumer"], "consumer-ws")
        self.assertEqual(ws["expected_checks"], 6)
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
        self.assertIn('union-default = ["opi-ws/union-default"]', manifest)
        self.assertIn('union-c-zlib = ["opi-ws/union-c-zlib"]', manifest)
        source = (base / "src" / "main.rs").read_text(encoding="utf-8")
        for name in [
            "rfc-hello-decodes",
            "context-takeover-across-messages",
            "driver-echo-roundtrip",
            "close-handshake-masked-response",
            "strict-rejection-is-terminal",
            "cancelled-borrow-resumes",
        ]:
            self.assertIn(name, source, f"missing check {name}")

    def test_companion_graph_uses_extracted_opi_ws(self):
        ws = package_slice.FAMILIES["ws"]
        temp = tempfile.TemporaryDirectory(prefix="opi-ws-graph-test-")
        self.addCleanup(temp.cleanup)
        package = Path(temp.name).resolve() / "opi-ws-0.1.0"
        package.mkdir()
        (package / "Cargo.toml").write_text("", encoding="utf-8")
        metadata = graph_metadata(
            ws["profiles"][0]["graph_requirements"], package / "Cargo.toml"
        )
        # graph_metadata names the SDK entry opi-ai; rename it to opi-ws for
        # this companion case.
        metadata["packages"][0]["name"] = "opi-ws"
        graph = package_slice.check_family_graph(
            metadata, package, ws["profiles"][0]["graph_requirements"], "opi-ws"
        )
        self.assertEqual(graph["libz-rs-sys"]["version"], "0.6.8")
        self.assertEqual(graph["zlib-rs"]["version"], "0.6.8")

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


class WsBridgeGateTests(unittest.TestCase):
    def test_real_upgrade_gates_each_feature_graph_before_packaging(self):
        with tempfile.TemporaryDirectory(prefix="opi-ws-bridge-gates-") as temp:
            root = Path(temp).resolve()
            repo, target, scratch = root / "repo", root / "target", root / "proof"
            companion = repo / "companions/ws"
            companion.mkdir(parents=True)
            (companion / "Cargo.toml").write_text(
                '[package]\nname = "opi-ws"\nversion = "0.1.0"\n', encoding="utf-8")
            target.mkdir()
            (target / ".opi-cargo-cache.json").write_text(
                json.dumps({"workspace": str(repo)}), encoding="utf-8")

            def fake_process(command, **kwargs):
                # Stop at the packaging boundary; all real scheduling and log
                # persistence before it still run, without invoking Cargo.
                failed = command[:2] == ["cargo", "package"]
                if "ws_upgrade" in command:
                    self.assertEqual(kwargs["cwd"], repo)
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
            bridges = [step for step in steps if step["name"].startswith("bridge-check")]
            common = ["cargo", "test", "-p", "opi-ai", "--test", "ws_upgrade",
                      "--offline", "--locked"]
            expected = [
                ("bridge-check-ordinary", common),
                ("bridge-check-union-default", common + ["--features", "opi-ws/union-default"]),
                ("bridge-check-union-c-zlib", common + ["--features", "opi-ws/union-c-zlib"]),
            ]
            self.assertEqual([(step["name"], step["command"]) for step in bridges], expected)
            names = [step["name"] for step in steps]
            self.assertLess(names.index("companion-doc"), names.index(expected[0][0]))
            self.assertLess(names.index(expected[-1][0]), names.index("package"))
            for name, command in expected:
                self.assertEqual((scratch / f"{name}.stdout").read_text(encoding="utf-8"),
                                 "stdout " + json.dumps(command))
                self.assertEqual((scratch / f"{name}.stderr").read_text(encoding="utf-8"),
                                 "stderr " + json.dumps(command))


if __name__ == "__main__":
    unittest.main()
