"""Offline packaged-opi-ai family proof for workspace-external consumers.

The generic entry point behind `--family <name>`: it packages `opi-ai` with
Cargo, safely extracts the artifact into a new external scratch, resolves an
ordinary consumer offline, validates its dependency identities/features and
runs the family's public-consumer profiles.

Families:

- ``ws`` (task 20.15, RDF-24): the standalone neutral owned WS companion
  (``companions/ws``, not a root workspace member) runs its own
  fmt/clippy/all-target test/doc gates in all three feature graphs, the
  strict Upgrade->Connection bridge test runs in the root workspace in each
  supported feature graph, and the
  real ``cargo package`` artifact of ``opi-ws`` is consumed by fresh
  workspace-external ordinary / union-default / union-c-zlib consumers whose
  resolved compression identities (libz-rs-sys/zlib-rs, or libz-sys for the
  C-zlib union) and native libz-sys build provenance are recorded. The root
  workspace full gate alone cannot cover this nonmember package.

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
        "kind": "companion",
        "crate": "opi-ws",
        "package_dir": "companions/ws",
        "consumer": "consumer-ws",
        "profiles": [
            {
                "name": "ordinary",
                "features": [],
                "graph_requirements": [
                    ("libz-rs-sys", "0.6.8"),
                    ("zlib-rs", "0.6.8"),
                    ("flate2", "1.1.10"),
                ],
            },
            {
                "name": "union-default",
                "features": ["union-default"],
                "graph_requirements": [
                    ("libz-rs-sys", "0.6.8"),
                    ("zlib-rs", "0.6.8"),
                    ("flate2", "1.1.10"),
                ],
            },
            {
                "name": "union-c-zlib",
                "features": ["union-c-zlib"],
                "graph_requirements": [
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
    native_closure = {"rustls", "aws-lc-rs", "aws-lc-sys", "libz-rs-sys", "zlib-rs", "libz-sys"}
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
        # The WS decoder consumes the hidden API. HTTP gzip's independent
        # compression graph needs only the standard Rust allocator/runtime.
        profile_features = (selected | {"__internal-api"}
                            if name == "zlib-rs" and crate != "opi-ws" else selected)
        if allowed is not None and profile_features != allowed:
            raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
        if name == "reqwest" and any(
            "native-tls" in feature or "native-roots" in feature or feature == "default-tls"
            for feature in selected
        ):
            raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
        if name == "flate2":
            if any("zlib-ng" in feature or "cloudflare" in feature for feature in selected):
                raise ValueError(f"unreviewed {name} feature profile: {sorted(selected)}")
            if crate == "opi-ws" and profile_name is not None:
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
        configs = check_cargo_configs(env, [REPO, scratch / "consumer", REPO / "companions/ws"])
        shared.validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    # The companion doc gate fails on warnings like the root workspace gate.
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
        if family.get("kind") == "companion":
            companion = REPO / family["package_dir"]
            companion_pkg = tomllib.loads(
                (companion / "Cargo.toml").read_text(encoding="utf-8")
            )["package"]
            prefix = f"{companion_pkg['name']}-{companion_pkg['version']}"
            # Standalone nonmember gates in every feature graph. The root
            # workspace full gate cannot see this package.
            run("companion-fmt", ["cargo", "fmt", "--check"], companion)
            for profile in family["profiles"]:
                tag = profile["name"]
                flags = (
                    ["--features", ",".join(profile["features"])]
                    if profile["features"] else []
                )
                run(
                    f"companion-clippy-{tag}",
                    ["cargo", "clippy", "--all-targets", "--offline", "--locked"]
                    + flags
                    + ["--", "-D", "warnings"],
                    companion,
                )
                run(
                    f"companion-test-{tag}",
                    ["cargo", "test", "--offline", "--locked"] + flags,
                    companion,
                )
            run(
                "companion-doc",
                ["cargo", "doc", "--no-deps", "--offline", "--locked"],
                companion,
            )
            # The root bridge must exercise the same companion feature union.
            for profile in family["profiles"]:
                features = [f"{family['crate']}/{feature}" for feature in profile["features"]]
                flags = ["--features", ",".join(features)] if features else []
                run(
                    f"bridge-check-{profile['name']}",
                    ["cargo", "test", "-p", "opi-ai", "--test", "ws_upgrade",
                     "--offline", "--locked"] + flags,
                    REPO,
                )
            run("package", ["cargo", "package", "--allow-dirty", "--no-verify",
                            "--offline", "--locked"], companion)
        else:
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
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
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
            )
            messages = [json.loads(line) for line in run(
                f"{profile['name']}-build",
                ["cargo", "build", "--message-format=json", "--offline", "--locked"] + flags,
                consumer,
            ).splitlines() if line.strip()]
            native_name = "libz-sys" if family.get("kind") == "companion" else "aws-lc-sys"
            native_packages = [p for p in metadata["packages"] if p["name"] == native_name]
            provenance = None
            if native_packages:
                provenance = native_build_provenance(target, messages, native_packages[0]["id"], native_name)
                output_snapshot = scratch / f"{profile['name']}-native-output.txt"
                shutil.copyfile(Path(provenance["out_dir"]).parent / "output", output_snapshot)
                provenance["output_snapshot"] = str(output_snapshot)
            elif family.get("kind") != "companion" or profile["name"] == "union-c-zlib":
                raise ValueError(f"missing current consumer {native_name} package")
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
