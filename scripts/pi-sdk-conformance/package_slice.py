"""Offline packaged-opi-ai family proof for workspace-external consumers.

The generic entry point behind `--family <name>`: it packages `opi-ai` with
Cargo, safely extracts the artifact into a new external scratch, resolves an
ordinary consumer offline, validates its dependency identities/features and
runs the family's public-consumer profiles.

Families:

- ``ws`` (RDF-24): the actual ``opi-ai`` package contains the private owned
  frame/PMD/Close/async-IO implementation and its retained protocol tests.
  Those tests and clippy run from the extracted artifact in all three
  feature graphs. Separate workspace-external consumers exercise only the
  existing public HTTP Upgrade and detached raw stream. Private tests do
  not prove a public provider/codec chain; that acceptance remains OPEN for
  task 20.16. Both AWS-LC and, in the C-zlib union, libz-sys native provenance
  are recorded from the consumer's actual Cargo units.

- ``crypto`` (task 20.14, RDF-23): the `consumer-crypto` template exercises
  the real public TLS/Models/Chat path (trust modes, mTLS, ALPN selection,
  typed negatives) in an ordinary profile and a caller feature-union
  profile that installs a process-global ring provider. Resolved rustls /
  aws-lc-rs / aws-lc-sys versions, sources, checksums and the actual native
  build provenance are recorded in the evidence file.

This is a bounded package proof, not the final SDK distribution acceptance
(task 20.57). There is no registry publication, live model call, patch,
registry server, source replacement or build-time generator. The scratch is
never deleted or reused by this command.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
from pathlib import Path, PurePosixPath
from pathlib import Path as FsPath
from typing import Any

import run_early_package_proof as shared

HARNESS = shared.HARNESS
REPO = shared.REPO
CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"

# Reviewed RDF-22/23/24 release identities, independent of the fresh consumer lock.
CHECKSUMS = {
    "aws-lc-rs": "faac5829c2b74c28f830747e7818ccfb684261b5f48a1118b1e2a13d36dfab13",
    "aws-lc-sys": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
    "flate2": "6e634e2e0ebac1ee034020da1ca582e17ffe4e0f5e985823721e168928136dcb",
    "http": "e3ba2a386d7f85a81f119ad7498ebe444d2e22c2af0b86b069416ace48b3311a",
    "hyper": "6299f016b246a94207e63da54dbe807655bf9e00044f73ded42c3ac5305fbcca",
    "hyper-util": "96547c2556ec9d12fb1578c4eaf448b04993e7fb79cbaad930a656880a6bdfa0",
    "libz-rs-sys": "434324ee61a8e4a05dd3fdf03d51c073ea7fdaf5a35b2ca33c4ca9f3f37def4a",
    "reqwest": "eddd3ca559203180a307f12d114c268abf583f59b03cb906fd0b3ff8646c1147",
    "rustls": "ef86cd5876211988985292b91c96a8f2d298df24e75989a43a3c73f2d4d8168b",
    "zlib-rs": "b268e58e7c693d7c271f93ffc4ba3b380412554231c85bf61ca7af91042a4112",
    "libz-sys": "f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19",
}


def controlled_environment(environment: dict[str, str], target: Path) -> dict[str, str]:
    """Reject source/link replacement inputs; retain ordinary compiler selection."""
    controlled = {"AWS_LC_SYS_USE_SYSTEM": "0", "LIBZ_SYS_STATIC": "1",
                  "VCPKGRS_DISABLE": "1", "CC_ENABLE_DEBUG_OUTPUT": "1"}
    for name, value in environment.items():
        upper = name.upper()
        if name in controlled and value == controlled[name]:
            continue
        if upper.startswith("CARGO_TARGET_") and upper.endswith("_LINKER"):
            continue
        if (upper.startswith(("AWS_LC_SYS_", "AWS_LC_FIPS_SYS_", "LIBZ_SYS_",
                              "CARGO_TARGET_", "CARGO_SOURCE_", "CARGO_REGISTRIES_"))
                or upper in {"RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER",
                             "RUSTC_WORKSPACE_WRAPPER", "CARGO_BUILD_TARGET",
                             "CARGO_BUILD_RUSTFLAGS", "CARGO_BUILD_RUSTC_WRAPPER",
                             "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER", "VCPKGRS_DISABLE"}):
            # CARGO_TARGET_DIR selects the existing shared cache, not native inputs.
            if upper != "CARGO_TARGET_DIR":
                raise ValueError(f"unreviewed package-proof override: {name}")
    return dict(environment, **controlled, CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="1")


def check_cargo_configs(environment: dict[str, str], directories: list[Path]) -> list[dict]:
    """Record effective config candidates and reject source/links/flag overrides.

    Cargo's home and ancestor configs can substitute sources or skip a build
    script even when metadata retains the crates.io package identity.
    """
    home = Path(environment.get("CARGO_HOME", str(Path.home() / ".cargo")))
    candidates = {home / name for name in ("config", "config.toml")}
    for directory in directories:
        for parent in [directory, *directory.parents]:
            candidates.update(parent / ".cargo" / name for name in ("config", "config.toml"))
    result = []
    for path in sorted(candidates):
        if not path.is_file():
            continue
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        if any(key in data for key in ("source", "patch", "replace", "registries")):
            raise ValueError(f"unreviewed source/target Cargo config: {path}")
        for config in data.get("target", {}).values():
            if "rustflags" in config or any(isinstance(value, dict) for value in config.values()):
                raise ValueError(f"unreviewed native links/flags Cargo config: {path}")
        build = data.get("build", {})
        if any(key in build for key in ("target", "rustflags", "rustc-wrapper", "rustc-workspace-wrapper")):
            raise ValueError(f"unreviewed build Cargo config: {path}")
        config_env = {}
        for name, value in data.get("env", {}).items():
            config_env[name] = str(value.get("value", "")) if isinstance(value, dict) else str(value)
        controlled_environment(config_env, Path(environment["CARGO_TARGET_DIR"]))
        result.append({"file": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
    return result

FAMILIES: dict[str, dict[str, Any]] = {
    "crypto": {
        "consumer": "consumer-crypto",
        "profiles": [
            {"name": "ordinary", "features": []},
            {"name": "caller-union", "features": ["caller-union"]},
        ],
        "expected_checks": 6,
        # Exact resolved identities the consumer graph must pin for the
        # packaged SDK (RDF-23 baseline).
        "graph_requirements": [
            ("rustls", "0.23.40"),
            ("aws-lc-rs", "1.18.2"),
            ("aws-lc-sys", "0.46.0"),
        ],
    },
    "http": {
        "consumer": "consumer-http",
        "profiles": [
            {"name": "ordinary", "features": []},
            {"name": "caller-union", "features": ["caller-union"]},
        ],
        "expected_checks": 6,
        # Exact resolved identities the consumer graph must pin for the
        # packaged SDK (RDF-22 baseline).
        "graph_requirements": [
            ("reqwest", "0.12.28"),
            ("hyper", "1.9.0"),
            ("hyper-util", "0.1.20"),
            ("http", "1.4.0"),
        ],
    },
    "ws": {
        "crate": "opi-ai",
        "private_test_minimum": 108,
        "public_codec_status": "OPEN: task 20.16",
        "consumer": "consumer-ws",
        "profiles": [
            {
                "name": "ordinary",
                "features": [],
                "graph_requirements": [
                    ("rustls", "0.23.40"),
                    ("aws-lc-rs", "1.18.2"),
                    ("aws-lc-sys", "0.46.0"),
                    ("libz-rs-sys", "0.6.8"),
                    ("zlib-rs", "0.6.8"),
                    ("flate2", "1.1.10"),
                ],
            },
            {
                "name": "union-default",
                "features": ["union-default"],
                "graph_requirements": [
                    ("rustls", "0.23.40"),
                    ("aws-lc-rs", "1.18.2"),
                    ("aws-lc-sys", "0.46.0"),
                    ("libz-rs-sys", "0.6.8"),
                    ("zlib-rs", "0.6.8"),
                    ("flate2", "1.1.10"),
                ],
            },
            {
                "name": "union-c-zlib",
                "features": ["union-c-zlib"],
                "graph_requirements": [
                    ("rustls", "0.23.40"),
                    ("aws-lc-rs", "1.18.2"),
                    ("aws-lc-sys", "0.46.0"),
                    ("libz-rs-sys", "0.6.8"),
                    ("zlib-rs", "0.6.8"),
                    ("flate2", "1.1.10"),
                    ("libz-sys", "1.1.30"),
                ],
            },
        ],
        "expected_checks": 6,
    },
}


def check_family_graph(
    metadata: dict, package: Path, requirements: list[tuple[str, str]], crate: str = "opi-ai",
    lock: dict | None = None, profile_name: str | None = None,
    ws_profile: bool = False,
) -> dict:
    packages = metadata["packages"]
    if any("fips" in p["name"] for p in packages):
        raise ValueError("FIPS packages are outside the reviewed standard profile")
    for entry in packages:
        if entry.get("source") is not None and entry["source"] != CRATES_IO:
            raise ValueError(f"unreviewed source for {entry['name']}: {entry['source']}")
    sdk = [p for p in packages if p["name"] == crate]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must compile the extracted opi-ai package")
    allowed_local = {sdk[0]["id"], metadata["resolve"].get("root")}
    for entry in packages:
        if entry.get("source") is None and entry.get("id") not in allowed_local:
            raise ValueError(f"unreviewed local dependency: {entry['name']}")
    features = {node["id"]: node["features"] for node in metadata["resolve"]["nodes"]}
    reviewed = dict(requirement for family in FAMILIES.values()
                    for profile in [family, *family["profiles"]]
                    for requirement in profile.get("graph_requirements", []))
    required = dict(requirements)
    native_closure = {"rustls", "aws-lc-rs", "aws-lc-sys", "flate2", "libz-rs-sys", "zlib-rs", "libz-sys"}
    required.update({p["name"]: reviewed[p["name"]] for p in packages if p["name"] in native_closure})
    result = {}
    for name, version in required.items():
        matches = [p for p in packages if p["name"] == name]
        if len(matches) != 1 or matches[0]["version"] != version or matches[0]["source"] != CRATES_IO:
            raise ValueError(
                f"expected one registry {name} {version}, found "
                + ", ".join(f"{p['version']}/{p.get('source')}" for p in matches)
            )
        entry = matches[0]
        checksum = entry.get("checksum")
        if lock is not None:
            locked = [p for p in lock.get("package", [])
                      if (p["name"], p["version"], p.get("source")) == (name, version, CRATES_IO)]
            checksum = locked[0].get("checksum") if len(locked) == 1 else None
        if checksum != CHECKSUMS[name]:
            raise ValueError(f"unreviewed or missing crates.io checksum for {name} {version}")
        result[name] = {
            "version": entry["version"],
            "source": entry["source"],
            "checksum": checksum,
            "features": features.get(entry["id"], []),
        }
        selected = set(result[name]["features"])
        allowed = {
            "aws-lc-sys": {"prebuilt-nasm"},
            "aws-lc-rs": {"alloc", "aws-lc-sys", "prebuilt-nasm"},
            "libz-rs-sys": {"std", "rust-allocator"},
            "zlib-rs": {"std", "rust-allocator", "__internal-api"},
            "libz-sys": set(),
        }.get(name)
        # The private WS decoder is a normal SDK dependency in every family;
        # record the real feature set, including its required hidden API.
        if allowed is not None and selected != allowed:
            raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
        if name == "reqwest" and any(
            "native-tls" in feature or "native-roots" in feature or feature == "default-tls"
            for feature in selected
        ):
            raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
        if name == "flate2":
            if any("zlib-ng" in feature or "cloudflare" in feature for feature in selected):
                raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
            if ws_profile and profile_name is not None:
                base = {"any_impl", "any_zlib", "zlib-rs"}
                profiles = {
                    "ordinary": base,
                    "union-default": base | {"default", "miniz_oxide", "runtime_detection", "rust_backend"},
                    "union-c-zlib": base | {"any_c_zlib", "libz-sys", "zlib"},
                }
                if selected != profiles.get(profile_name):
                    raise ValueError(f"unreviewed {name} {profile_name} feature profile: {sorted(selected)}")
    return result


def check_family_report(report: dict, family: dict) -> None:
    if not isinstance(report, dict) or not isinstance(report.get("checks"), list):
        raise ValueError("consumer report must be an object with a checks list")
    checks = report["checks"]
    if any(not isinstance(check, dict) or not isinstance(check.get("name"), str) for check in checks):
        raise ValueError("consumer report must name every check")
    expected = family["expected_checks"]
    if (
        report.get("verdict") != "PASS"
        or report.get("failed") != 0
        or report.get("total") != expected
        or len(checks) != expected
        or len({c["name"] for c in checks if isinstance(c, dict)}) != expected
        or any(c.get("passed") is not True for c in checks)
    ):
        raise ValueError(
            f"consumer did not pass the complete {expected}-check family battery: "
            + json.dumps(report)[:800]
        )


def native_build_provenance(
    target: Path, messages: list[dict] | None = None, package_id: str | None = None,
    name: str = "aws-lc-sys",
) -> dict:
    """Validate the native unit selected by this consumer's Cargo build.

    JSON build-script messages also describe reused units. Only that exact
    unit's output/fingerprint/link metadata and referenced archive count as
    evidence; unrelated cache contents say nothing about the current link.
    """
    if not messages or not package_id:
        raise ValueError("native provenance requires the current consumer Cargo messages/package ID")
    scripts = [m for m in messages if m.get("reason") == "build-script-executed"
               and m.get("package_id") == package_id]
    artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                 and m.get("package_id") == package_id and "lib" in m.get("target", {}).get("kind", [])]
    if len(scripts) != 1 or len(artifacts) != 1:
        raise ValueError(f"missing or ambiguous consumer {name} native provenance")
    expected_features = ["prebuilt-nasm"] if name == "aws-lc-sys" else []
    if artifacts[0].get("features") != expected_features:
        raise ValueError(f"unreviewed consumer {name} compiler features")
    manifest = Path(artifacts[0].get("manifest_path", "")).resolve()
    source = manifest.parent
    source_archive = source.parents[2] / "cache" / source.parent.name / f"{source.name}.crate"
    if (not source_archive.is_file()
            or hashlib.sha256(source_archive.read_bytes()).hexdigest() != CHECKSUMS[name]):
        raise ValueError(f"unreviewed or absent {name} source checksum")
    source_count = 0
    with tarfile.open(source_archive, "r:gz") as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            if (not parts or parts[0] != source.name or ".." in parts
                    or not (member.isdir() or member.isfile())):
                raise ValueError(f"unreviewed {name} source archive member")
            if member.isfile():
                path = source.joinpath(*parts[1:])
                if (not path.resolve().is_relative_to(source) or not path.is_file()
                        or path.read_bytes() != archive.extractfile(member).read()):
                    raise ValueError(f"{name} source differs from reviewed archive: {path}")
                source_count += 1
    source_manifest = tomllib.loads(manifest.read_text(encoding="utf-8"))
    source_files = [manifest, Path(artifacts[0]["target"]["src_path"]).resolve(),
                    (source / source_manifest["package"].get("build", "build.rs")).resolve()]
    if any(not path.is_relative_to(source) or not path.is_file() for path in source_files):
        raise ValueError(f"missing or external consumer {name} source provenance")
    script = scripts[0]
    out = Path(script.get("out_dir", "")).resolve()
    build = out.parent
    profile = target.resolve() / "debug"
    if out.name != "out" or build.parent != profile / "build" or not build.name.startswith(name + "-"):
        raise ValueError(f"unexpected consumer {name} out_dir: {out}")
    fingerprint_dir = profile / ".fingerprint" / build.name
    fingerprints = list(fingerprint_dir.glob("run-build-script-*.json"))
    if len(fingerprints) != 1:
        raise ValueError(f"missing or ambiguous consumer {name} fingerprint")
    fingerprint_path = fingerprints[0]
    fingerprint = json.loads(fingerprint_path.read_text(encoding="utf-8"))
    if fingerprint.get("rustflags") != []:
        raise ValueError(f"unreviewed {name} fingerprint rustflags")
    inputs = {}
    for entry in fingerprint.get("local", []):
        if "RerunIfEnvChanged" in entry:
            item = entry["RerunIfEnvChanged"]
            inputs[item["var"]] = item["val"]
    controlled_environment({key: val for key, val in inputs.items() if val is not None}, target)
    required = ("AWS_LC_SYS_USE_SYSTEM", "0") if name == "aws-lc-sys" else ("LIBZ_SYS_STATIC", "1")
    if inputs.get(required[0]) != required[1]:
        raise ValueError(f"{name} provenance lacks controlled {required[0]}={required[1]}")
    output_path = build / "output"
    output = output_path.read_text(encoding="utf-8")
    compiler_invocations = [line for line in output.splitlines() if line.startswith("running:")]
    if inputs.get("CC_ENABLE_DEBUG_OUTPUT") != "1" or not compiler_invocations:
        raise ValueError(f"missing current consumer {name} compiler invocation evidence")
    expected_lib = "aws_lc_0_46_0_crypto" if name == "aws-lc-sys" else "z"
    linked_libs = script.get("linked_libs", [])
    if linked_libs != [f"static={expected_lib}"]:
        raise ValueError(f"unreviewed {name} native link libraries: {linked_libs}")
    if f"cargo:rustc-link-lib=static={expected_lib}" not in output.splitlines():
        raise ValueError(f"{name} output disagrees with consumer native link metadata")
    search = [Path(path.removeprefix("native=")).resolve()
              for path in script.get("linked_paths", [])]
    if not search or any(not path.is_relative_to(out) for path in search):
        raise ValueError(f"unreviewed {name} native link search paths: {search}")
    archives = {candidate.resolve() for directory in search
                for filename in (f"lib{expected_lib}.a", f"{expected_lib}.lib")
                if (candidate := directory / filename).is_file()}
    if not archives:
        raise ValueError(f"missing or ambiguous consumer-linked {name} static archive")
    archive_bytes = {path: path.read_bytes() for path in archives}
    if (len(set(archive_bytes.values())) != 1
            or any(not data.startswith(b"!<arch>\n") or len(data) <= 8 for data in archive_bytes.values())):
        raise ValueError(f"invalid or ambiguous consumer-linked {name} static archive")
    builders = [line.removeprefix("cargo:warning=Building with: ") for line in output.splitlines()
                if line.startswith("cargo:warning=Building with:")]
    if name == "aws-lc-sys" and builders != ["CC"]:
        raise ValueError(f"unreviewed aws-lc-sys builder provenance: {builders}")
    files = [*artifacts[0].get("filenames", []), *map(str, sorted(archives)),
             str(output_path), str(fingerprint_path)]
    if not artifacts[0].get("filenames"):
        raise ValueError(f"missing consumer {name} compiler artifacts")
    hashes = {}
    for filename in files:
        path = Path(filename).resolve()
        if not path.is_relative_to(profile) or not path.is_file():
            raise ValueError(f"missing or external consumer {name} artifact: {path}")
        hashes[str(path)] = hashlib.sha256(path.read_bytes()).hexdigest()
    return {
        "package_id": package_id, "out_dir": str(out), "builders": builders,
        "static_archives": list(map(str, sorted(archives))), "linked_libs": linked_libs,
        "linked_paths": script["linked_paths"], "fingerprint_inputs": inputs,
        "artifact_sha256": hashes, "compiler_artifact": artifacts[0],
        "source_sha256": {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in source_files},
        "source_package_checksum": CHECKSUMS[name],
        "source_archive": str(source_archive), "verified_source_files": source_count,
        "compiler_invocations": compiler_invocations,
        "scope": "current consumer Cargo unit, build output and static link metadata; not binary inspection",
    }


def zlib_native_provenance(target: Path, messages: list[dict] | None = None,
                           package_id: str | None = None) -> dict:
    return native_build_provenance(target, messages, package_id, "libz-sys")


WS_PACKAGE_FILES = (
    "Cargo.toml", "LICENSE", "NOTICE", "src/ws.rs", "src/ws/frame.rs", "src/ws/pmd.rs", "src/ws/io.rs",
    "src/ws/tests/mod.rs", "src/ws/tests/upgrade.rs", "src/ws/tests/packaged_cases.rs",
    "src/ws/tests/deflate_audit.rs", "src/ws/tests/tls_fixtures.rs",
    "src/ws/tests/fixtures/extract-boundary-fixtures.py",
    *(f"src/ws/tests/{name}.rs" for name in
      ("boundary", "close", "frame", "gaps", "io", "pmd", "pmd_async", "pmd_red", "replay")),
    *(f"src/ws/tests/fixtures/{name}.json" for name in
      ("boundary-expectations", "boundary-seeds", "memory-codec-cases", "pmd-context-fixtures",
       "python-pmd-fixtures")),
)


def check_ws_package(package: Path) -> dict[str, str]:
    """Require the retained private implementation/tests inside the real SDK."""
    for name in WS_PACKAGE_FILES:
        if not (package / name).is_file():
            raise ValueError(f"missing private WS package file: {name}")
    manifest = tomllib.loads((package / "Cargo.toml").read_text(encoding="utf-8"))
    if manifest.get("package", {}).get("name") != "opi-ai":
        raise ValueError("private WS proof requires the extracted opi-ai package")
    if any("opi-ws" in manifest.get(key, {})
           for key in ("dependencies", "dev-dependencies", "build-dependencies")):
        raise ValueError("retired companion dependency in the SDK package")
    paths = [package / "src/ws.rs", *sorted((package / "src/ws").rglob("*.rs"))]
    for path in paths:
        text = path.read_text(encoding="utf-8")
        if "opi_ws::" in text or "companions/ws" in text.replace("\\", "/"):
            raise ValueError(f"retired companion reference in {path}")
        # This controlled tree has no path attributes. Includes use only a
        # single unescaped ordinary string in parentheses, so every allowed
        # reference can be checked without evaluating arbitrary Rust syntax.
        if re.search(r'#\s*\[[^\]]*\bpath\s*=', text):
            raise ValueError(f"external path attribute is not allowed in private WS source: {path}")
        for include in re.finditer(r'\binclude(?:_bytes|_str)?\s*!', text):
            literal = re.match(r'\s*\(\s*"([^"\\\n]+)"\s*\)', text[include.end():])
            if literal is None:
                raise ValueError(f"unverifiable private WS include argument: {path}")
            reference = literal[1]
            destination = (path.parent / reference).resolve()
            if not destination.is_relative_to(package) or not destination.is_file():
                raise ValueError(f"external or missing private WS source/fixture: {path}: {reference}")
    return {name: hashlib.sha256((package / name).read_bytes()).hexdigest()
            for name in WS_PACKAGE_FILES}


def check_ws_consumer(consumer: Path, package: Path) -> None:
    """External checks use the public SDK, never a copied/private codec seam."""
    manifest = tomllib.loads((consumer / "Cargo.toml").read_text(encoding="utf-8"))
    sdk = manifest.get("dependencies", {}).get("opi-ai", {})
    if (not isinstance(sdk, dict) or "path" not in sdk
            or (consumer / sdk["path"]).resolve() != package):
        raise ValueError("WS consumer must use the extracted opi-ai package")
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for name, dependency in manifest.get(section, {}).items():
            if name == "opi-ws" or (name != "opi-ai" and isinstance(dependency, dict)
                                    and "path" in dependency):
                raise ValueError("WS consumer must use only the extracted opi-ai local package")
    if any(key in manifest for key in ("patch", "replace")):
        raise ValueError("WS consumer must not replace production source")
    for path in (consumer / "src").rglob("*.rs"):
        text = path.read_text(encoding="utf-8")
        # The controlled template has no path attributes or include macros.
        # Also reject nested cfg_attr paths and legal whitespace before `!`;
        # this guard is not a parser for arbitrary Rust source.
        if (re.search(r'#\s*\[[^\]]*\bpath\s*=|\binclude(?:_bytes|_str)?\s*!|\bopi_ws::', text)
                or "companions/ws" in text.replace("\\", "/")):
            raise ValueError(f"external WS consumer must not include production source: {path}")


def check_private_ws_report(output: str, minimum: int) -> int:
    summaries = re.findall(r"test result: ok\. (\d+) passed; (\d+) failed;", output)
    if len(summaries) != 1 or int(summaries[0][0]) < minimum or int(summaries[0][1]) != 0:
        raise ValueError(f"packaged private WS battery must pass at least {minimum} tests")
    return int(summaries[0][0])


def run_packaged_ws_checks(
    run: Any, package: Path, family: dict, caller_checksum_lock_path: Path,
) -> list[dict]:
    """Run private tests from package bytes without adding a public test API."""
    profiles = []
    lock_path = package / "Cargo.lock"
    packaged_lock_sha256 = (hashlib.sha256(lock_path.read_bytes()).hexdigest()
                            if lock_path.is_file() else None)
    caller_lock_bytes = caller_checksum_lock_path.read_bytes()
    caller_lock = tomllib.loads(caller_lock_bytes.decode("utf-8"))
    caller_checksum_lock = {
        "file": str(caller_checksum_lock_path),
        "sha256": hashlib.sha256(caller_lock_bytes).hexdigest(),
        "scope": "independent caller registry checksums; not the private test build lock",
    }
    for profile in family["profiles"]:
        tag = profile["name"]
        feature = {"ordinary": None, "union-default": "flate2/default",
                   "union-c-zlib": "flate2/zlib"}[tag]
        flags = ["--features", feature] if feature else []
        manifest = ["--manifest-path", str(package / "Cargo.toml")]
        # Cargo metadata may resolve a dependency feature union without
        # recording its optional native package in the library's lock.
        # Validate selected identities/features from this metadata against
        # the fresh caller's exact registry checksums, then separately audit
        # the library lock's actual coverage. Tests still use its own lock.
        metadata = json.loads(run(
            f"private-ws-metadata-{tag}",
            ["cargo", "metadata", *manifest, "--format-version", "1", "--offline", *flags], package))
        if not lock_path.is_file():
            raise ValueError(f"private WS {tag} metadata did not produce a Cargo.lock")
        graph = check_family_graph(metadata, package, profile["graph_requirements"],
                                   lock=caller_lock, profile_name=tag, ws_profile=True)
        library_lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
        library_lock_missing = []
        for name, reviewed in graph.items():
            entries = [entry for entry in library_lock.get("package", [])
                       if (entry["name"], entry["version"]) == (name, reviewed["version"])]
            if not entries:
                library_lock_missing.append(name)
            elif (len(entries) != 1 or entries[0].get("source") != reviewed["source"]
                  or entries[0].get("checksum") != reviewed["checksum"]):
                raise ValueError(f"unreviewed private WS library lock entry for {name}")
        output = run(f"private-ws-test-{tag}",
                     ["cargo", "test", *manifest, "--lib", "ws::", "--offline", "--locked", *flags],
                     package)
        passed = check_private_ws_report(output, family["private_test_minimum"])
        run(f"private-ws-clippy-{tag}",
            ["cargo", "clippy", *manifest, "--lib", "--offline", "--locked", *flags,
             "--", "-D", "warnings"], package)
        profiles.append({"name": tag, "passed": passed,
                         "graph": graph,
                         "packaged_library_lock_sha256": packaged_lock_sha256,
                         "resolved_library_lock_sha256": hashlib.sha256(lock_path.read_bytes()).hexdigest(),
                         "resolved_library_lock_missing_reviewed_identities": library_lock_missing,
                         "caller_checksum_lock": caller_checksum_lock,
                         "scope": "extracted SDK private protocol/Upgrade tests; not a public consumer"})
    return profiles


def collect_native_provenance(
    target: Path, messages: list[dict], metadata: dict, require_zlib: bool = False,
) -> dict[str, dict]:
    names = ["aws-lc-sys"]
    if require_zlib or any(p["name"] == "libz-sys" for p in metadata["packages"]):
        names.append("libz-sys")
    result = {}
    for name in names:
        packages = [p for p in metadata["packages"] if p["name"] == name]
        if len(packages) != 1:
            raise ValueError(f"missing or ambiguous current consumer {name} package")
        result[name] = native_build_provenance(target, messages, packages[0]["id"], name)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family", required=True, choices=sorted(FAMILIES))
    parser.add_argument("--offline", action="store_true", help="required offline mode marker")
    parser.add_argument("--scratch", required=True, help="new workspace-external scratch directory")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR"),
                        help="existing marked workspace cache (or CARGO_TARGET_DIR)")
    args = parser.parse_args()
    if not args.offline:
        parser.error("--offline is required; this proof never resolves from the network")
    if not args.target_dir:
        parser.error("--target-dir or CARGO_TARGET_DIR must identify the existing workspace cache")
    family = FAMILIES[args.family]
    scratch = FsPath(args.scratch).expanduser().resolve()
    target = FsPath(args.target_dir).expanduser().resolve()
    try:
        env = controlled_environment(dict(os.environ), target)
        configs = check_cargo_configs(env, [REPO, scratch / "consumer", scratch / "package"])
        shared.validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    # Package rustdoc uses the same warnings gate as the root workspace.
    env.setdefault("RUSTDOCFLAGS", "-D warnings")
    env.setdefault("CARGO_BUILD_JOBS", "4")
    evidence: dict[str, Any] = {
        "family": args.family,
        "scope": "packaged public-consumer family proof; not final SDK acceptance",
        "target": str(target),
        "cargo_configs": configs,
        "controlled_native_inputs": {key: env[key] for key in
                                     ("AWS_LC_SYS_USE_SYSTEM", "LIBZ_SYS_STATIC", "VCPKGRS_DISABLE",
                                      "CC_ENABLE_DEBUG_OUTPUT")},
        "steps": [],
        "profiles": [],
        "verdict": "FAIL",
        "inputs": {
            str(path.relative_to(HARNESS)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in [
                *sorted((HARNESS / family["consumer"] / "src").glob("*.rs")),
                HARNESS / family["consumer"] / "Cargo.toml",
            ]
        },
    }
    if args.family == "ws":
        evidence["scope"] = "packaged SDK private WS tests plus external public Upgrade-only proof"
        evidence["public_codec_status"] = family["public_codec_status"]
        evidence["public_codec_exercised"] = False

    def run(name: str, command: list[str], cwd: Path) -> str:
        started = time.monotonic()
        result = subprocess.run(
            command, cwd=cwd, env=env, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=3600,
        )
        (scratch / f"{name}.stdout").write_text(result.stdout, encoding="utf-8")
        (scratch / f"{name}.stderr").write_text(result.stderr, encoding="utf-8")
        evidence["steps"].append({
            "name": name, "command": command, "exit": result.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 2),
        })
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}; see scratch logs")
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        evidence["rustc"] = run("rustc-version", ["rustc", "-vV"], REPO)
        evidence["cargo"] = run("cargo-version", ["cargo", "-vV"], REPO)
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        version = tomllib.loads(
            (REPO / "Cargo.toml").read_text(encoding="utf-8")
        )["workspace"]["package"]["version"]
        prefix = f"opi-ai-{version}"
        run("package", ["cargo", "package", "-p", "opi-ai", "--allow-dirty", "--no-verify",
                        "--offline", "--locked"], REPO)
        archive = scratch / f"{prefix}.crate"
        archive.write_bytes((target / "package" / archive.name).read_bytes())
        evidence["package"] = {
            "file": archive.name,
            "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        }
        package = shared.unpack_package(archive, scratch / "package", prefix)
        if args.family == "ws":
            evidence["private_ws_files"] = check_ws_package(package)
        consumer = scratch / "consumer"
        shutil.copytree(
            HARNESS / family["consumer"],
            consumer,
            ignore=shutil.ignore_patterns("target", "Cargo.lock"),
        )
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(
            text.replace('"@PACKAGE@"', json.dumps(package.as_posix())),
            encoding="utf-8", newline="\n",
        )
        if args.family == "ws":
            check_ws_consumer(consumer, package)
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
        if args.family == "ws":
            evidence["private_ws_profiles"] = run_packaged_ws_checks(
                run, package, family, consumer / "Cargo.lock")
            run("private-ws-doc", ["cargo", "doc", "--manifest-path", str(package / "Cargo.toml"),
                                   "--no-deps", "--offline", "--locked"], package)
        for profile in family["profiles"]:
            flags = ["--features", ",".join(profile["features"])] if profile["features"] else []
            metadata = json.loads(run(
                f"{profile['name']}-metadata",
                ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"] + flags,
                consumer,
            ))
            graph = check_family_graph(
                metadata, package,
                profile.get("graph_requirements", family.get("graph_requirements")),
                family.get("crate", "opi-ai"),
                lock,
                profile["name"],
                ws_profile=args.family == "ws",
            )
            messages = [json.loads(line) for line in run(
                f"{profile['name']}-build",
                ["cargo", "build", "--message-format=json", "--offline", "--locked"] + flags,
                consumer,
            ).splitlines() if line.strip()]
            provenance = collect_native_provenance(
                target, messages, metadata,
                require_zlib=args.family == "ws" and profile["name"] == "union-c-zlib")
            for native_name, native in provenance.items():
                output_snapshot = scratch / f"{profile['name']}-{native_name}-native-output.txt"
                shutil.copyfile(Path(native["out_dir"]).parent / "output", output_snapshot)
                native["output_snapshot"] = str(output_snapshot)
            executables = [m for m in messages if m.get("reason") == "compiler-artifact"
                           and m.get("executable") and m.get("target", {}).get("name") == family["consumer"]]
            if len(executables) != 1:
                raise ValueError("missing or ambiguous current consumer executable")
            executable = Path(executables[0]["executable"]).resolve()
            if not executable.is_relative_to(target) or not executable.is_file():
                raise ValueError("consumer executable is outside the shared target or missing")
            report = json.loads(run(
                f"{profile['name']}-consumer",
                [str(executable)],
                consumer,
            ))
            check_family_report(report, family)
            evidence["profiles"].append({
                "name": profile["name"],
                "scope": ("external public Upgrade and raw stream; private codec not exercised"
                          if args.family == "ws" else "external public consumer"),
                "graph": graph,
                "report": report,
                "native_provenance": provenance,
                "executable": {"file": str(executable),
                               "sha256": hashlib.sha256(executable.read_bytes()).hexdigest()},
            })
        evidence["consumer_lock_sha256"] = hashlib.sha256(
            (consumer / "Cargo.lock").read_bytes()
        ).hexdigest()
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
        graph_names = {
            name
            for profile in family["profiles"]
            for name, _ in profile.get(
                "graph_requirements", family.get("graph_requirements", [])
            )
        }
        lock_checksums = {
            p["name"]: p.get("checksum")
            for p in lock.get("package", [])
            if p["name"] in graph_names
        }
        evidence["lock_checksums"] = lock_checksums
        missing = [n for n, c in lock_checksums.items() if not c]
        if missing:
            raise ValueError(f"consumer lock lacks registry checksums for {missing}")
        evidence["native_provenance"] = {
            p["name"]: p["native_provenance"] for p in evidence["profiles"]
        }
        evidence["verdict"] = "PASS"
    except (OSError, ValueError, subprocess.TimeoutExpired, tarfile.TarError) as exc:
        evidence["error"] = str(exc)
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.TimeoutExpired) as exc:
                evidence["error"] = str(exc)
                evidence["verdict"] = "FAIL"
        (scratch / "evidence.json").write_text(
            json.dumps(evidence, indent=2) + "\n", encoding="utf-8"
        )
    print(json.dumps({
        "verdict": evidence["verdict"],
        "scratch": str(scratch),
        "error": evidence.get("error"),
    }, indent=2))
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
