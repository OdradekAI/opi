"""Verification-branch entry point; consumes an immutable production commit."""
import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import types

sys.dont_write_bytecode = True

from full_sdk_matrix import ProcessGuard, local_peer_environment, local_peer_environment_receipt


HTTP_CASES = [
    "redirect-policy-owned-default",
    "same-origin-redirect-preserves-authorization",
    "cross-origin-redirect-strips-authorization",
    "redirect-limit-typed-failure-exact-sends",
    "proxy-routing-with-scoped-bypass",
    "acquisition-failure-and-single-terminal-eof",
]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def retain_file(path, destination, expected):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(path, destination)
    observed = sha256(destination)
    if observed != expected:
        raise ValueError(f"retained bytes changed: {path}")
    return {"source": str(path), "file": str(destination), "sha256": observed}


def run_family_with_snapshots(family, name, scratch, target):
    """Freeze each validated profile before its next build overwrites top-level bins."""
    config = family.FAMILIES[name]
    snapshots = []
    original_check = family.check_family_report

    def retain_profile(profile):
        messages = [json.loads(line) for line in
                    (scratch / f"{profile}-build.stdout").read_text(encoding="utf-8").splitlines()
                    if line.strip()]
        metadata = json.loads((scratch / f"{profile}-metadata.stdout").read_text(encoding="utf-8"))
        binaries = [item for item in messages
                    if item.get("reason") == "compiler-artifact"
                    and item.get("executable")
                    and item.get("target", {}).get("name") == config["consumer"]]
        if len(binaries) != 1:
            raise ValueError("missing exact family consumer executable")
        binary = pathlib.Path(binaries[0]["executable"]).resolve()
        if not binary.is_relative_to(target.resolve()):
            raise ValueError("family executable outside selected cache")
        saved = scratch / "retained-artifacts" / profile
        executable = retain_file(binary, saved / binary.name, sha256(binary))
        provenance = family.collect_native_provenance(
            target, messages, metadata,
            require_zlib=name == "ws" and profile == "union-c-zlib")
        native_files = {}
        for native_name, native in provenance.items():
            required = dict(native["artifact_sha256"])
            required.update(native["source_sha256"])
            required[native["source_archive"]] = native["source_package_checksum"]
            retained = []
            for number, (path, expected) in enumerate(sorted(required.items())):
                path = pathlib.Path(path)
                retained.append(retain_file(path, saved / native_name / f"{number}-{path.name}", expected))
            native_files[native_name] = retained
        return {"name": profile, "executable": executable, "native_files": native_files,
                "native_provenance": provenance,
                "build_log_sha256": sha256(scratch / f"{profile}-build.stdout"),
                "metadata_sha256": sha256(scratch / f"{profile}-metadata.stdout")}

    def check_and_retain(report, actual_config):
        original_check(report, actual_config)
        if name == "http" and [check["name"] for check in report["checks"]] != HTTP_CASES:
            raise ValueError("actual HTTP consumer omitted or changed a required named case")
        index = len(snapshots)
        if actual_config is not config or index >= len(config["profiles"]):
            raise ValueError("unexpected family/profile report order")
        profile = config["profiles"][index]["name"]
        snapshots.append(retain_profile(profile))
        (scratch / "retained-profile-artifacts.json").write_text(
            json.dumps({"profiles": snapshots, "complete": False}, indent=2), encoding="utf-8")

    family.check_family_report = check_and_retain
    try:
        sys.argv = ["package_slice.py", "--family", name, "--offline", "--scratch", str(scratch),
                    "--target-dir", str(target)]
        result = family.main()
    finally:
        family.check_family_report = original_check
    if result:
        return result, None
    evidence_path = scratch / "evidence.json"
    evidence = json.loads(evidence_path.read_bytes())
    expected_names = [profile["name"] for profile in config["profiles"]]
    if ([profile["name"] for profile in evidence["profiles"]] != expected_names
            or [profile["name"] for profile in snapshots] != expected_names):
        raise ValueError("incomplete family snapshot profile coverage")
    for profile, retained in zip(evidence["profiles"], snapshots):
        if (profile["executable"]["file"] != retained["executable"]["source"]
                or profile["executable"]["sha256"] != retained["executable"]["sha256"]
                or sha256(pathlib.Path(retained["executable"]["file"])) != retained["executable"]["sha256"]):
            raise ValueError("executed family binary does not match retained profile")
        if set(profile["native_provenance"]) != set(retained["native_files"]):
            raise ValueError("native snapshot identity mismatch")
        for native_name, native in profile["native_provenance"].items():
            required = dict(native["artifact_sha256"])
            required.update(native["source_sha256"])
            required[native["source_archive"]] = native["source_package_checksum"]
            actual = {item["source"]: item["sha256"] for item in retained["native_files"][native_name]}
            if required != actual:
                raise ValueError("native snapshot does not match original provenance")
            for item in retained["native_files"][native_name]:
                if sha256(pathlib.Path(item["file"])) != item["sha256"]:
                    raise ValueError("retained native snapshot changed")
    snapshot_path = scratch / "retained-profile-artifacts.json"
    snapshot_path.write_text(json.dumps({"profiles": snapshots, "complete": True}, indent=2), encoding="utf-8")
    return 0, {"family": name, "evidence": str(evidence_path), "sha256": sha256(evidence_path),
               "retained_artifacts": str(snapshot_path), "retained_artifacts_sha256": sha256(snapshot_path)}


def sealed_source_inputs(repository, source, head):
    """Bind every exported leaf to its immutable Git blob and retain SHA-256s."""
    result = []
    names = set()
    tree = subprocess.check_output(["git", "ls-tree", "-r", "-z", head], cwd=repository)
    for entry in tree.split(b"\0"):
        if not entry:
            continue
        header, encoded_name = entry.split(b"\t", 1)
        mode, kind, blob = header.decode("ascii").split()
        name = encoded_name.decode("utf-8")
        names.add(name)
        path = source / name
        if kind != "blob" or mode not in {"100644", "100755", "120000"}:
            raise ValueError("unsupported sealed input mode: " + name)
        if mode == "120000":
            if not path.is_symlink():
                raise ValueError("sealed symbolic link was not preserved: " + name)
            data = os.fsencode(os.readlink(path))
        else:
            if not path.is_file() or path.is_symlink():
                raise ValueError("missing sealed regular file: " + name)
            data = path.read_bytes()
        actual_blob = hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()
        if actual_blob != blob:
            raise ValueError("sealed source differs from immutable Git blob: " + name)
        result.append({"path": name, "mode": mode, "git_blob": blob,
                       "sha256": hashlib.sha256(data).hexdigest()})
    observed = set()
    for parent, directories, files in os.walk(source, followlinks=False):
        for name in [*files, *(name for name in directories if (pathlib.Path(parent) / name).is_symlink())]:
            observed.add((pathlib.Path(parent) / name).relative_to(source).as_posix())
    if observed != names:
        raise ValueError("sealed source has missing or additional leaf inputs")
    return sorted(result, key=lambda item: item["path"])


def require_final_sdk(evidence, exit_code):
    """Final acceptance requires complete named execution with actual exit zero."""
    tests = ["external_sdk", "provider_collection", "oauth_flows", "deferred_contract", "classifier_contract"]
    profiles = evidence.get("profiles", [])
    if (exit_code != 0 or evidence.get("verdict") != "PASS"
            or evidence.get("evidence_status") != "COMPLETE"
            or evidence.get("tests") != tests
            or [profile.get("name") for profile in profiles] != ["ordinary", "default", "c-zlib"]):
        raise ValueError("full SDK matrix requires final PASS/exit0 and exact target/profile coverage")
    for profile in profiles:
        targets = profile.get("test_targets", [])
        if ([item.get("target") for item in targets] != tests
                or profile.get("failed") != 0 or profile.get("ignored") != 0
                or profile.get("failures") or profile.get("known_failures") or profile.get("unknown_failures")):
            raise ValueError("full SDK profile has missing targets or failed execution")
        executed = 0
        for target in targets:
            execution = target.get("execution", {})
            observed = execution.get("observed", {})
            if (target.get("test_exit") != 0 or target.get("checker_exit") != 0
                    or execution.get("status") != "PASS"
                    or not isinstance(observed.get("executed"), int) or observed["executed"] <= 0
                    or observed.get("failed") != 0 or observed.get("ignored") != 0):
                raise ValueError("full SDK target lacks successful named test execution")
            executables = target.get("executables", [])
            if len(executables) != 1 or sha256(pathlib.Path(executables[0]["file"])) != executables[0]["sha256"]:
                raise ValueError("full SDK retained executable identity changed")
            executed += observed["executed"]
        if profile.get("executed") != executed:
            raise ValueError("full SDK executed total does not match target observations")


def configure_readme_adapter(readme, source, out):
    """Keep the sealed runner immutable while redirecting its generated proof."""
    readme.REPO = source
    harness = out / "readme-harness"
    (harness / "evidence").mkdir(parents=True)
    shutil.copytree(source / "scripts/pi-sdk-conformance/consumer-readme",
                    harness / "consumer-readme",
                    ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    readme.HARNESS = harness
    runner = source / "scripts/pi-sdk-conformance/run_readme_examples.py"
    return {
        "scope": "runtime path adapter only; immutable exported runner and source bytes",
        "runner": str(runner), "runner_sha256": sha256(runner),
        "REPO": str(source), "HARNESS": str(harness),
        "validate_paths": "workspace-marked cache validated against repository; export and scratch remain disjoint",
        "template_sha256": {str(path.relative_to(harness)): sha256(path)
                            for path in sorted((harness / "consumer-readme").rglob("*"))
                            if path.is_file()},
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--prime-registry", action="store_true")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.head):
        parser.error("--head must be an exact full commit SHA")
    repository = pathlib.Path(args.repository).resolve()
    resolved_head = subprocess.check_output(["git", "rev-parse", args.head + "^{commit}"], cwd=repository, text=True).strip()
    if resolved_head != args.head:
        raise ValueError("production source commit did not resolve exactly")
    if subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repository, text=True).strip() != resolved_head:
        raise ValueError("verification checkout must be at the immutable tested commit")
    out = pathlib.Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=False)
    source = out / "source"
    source.mkdir()
    archive = out / "source.tar"
    guard = ProcessGuard()

    def run(command, cwd):
        print("RUN " + subprocess.list2cmdline(command), flush=True)
        return guard.run(command, cwd=cwd, check=True, timeout=3600)

    run(["git", "archive", "--format=tar", "-o", str(archive), args.head], repository)
    with tarfile.open(archive) as handle:
        handle.extractall(source, filter="data")
    target = pathlib.Path(subprocess.check_output([
        sys.executable, str(repository / "scripts/opi-cargo-cache.py"), "resolve", "--workspace", str(repository)
    ], cwd=repository, text=True).strip())
    run([sys.executable, str(repository / "scripts/opi-cargo-cache.py"), "status", "--target", str(target), "--quick", "--json"], repository)
    os.environ.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL="1", CARGO_BUILD_JOBS="4",
                      AWS_LC_SYS_USE_SYSTEM="0", LIBZ_SYS_STATIC="1", VCPKGRS_DISABLE="1", CC_ENABLE_DEBUG_OUTPUT="1",
                      PYTHONDONTWRITEBYTECODE="1")
    sys.path.insert(0, str(source / "scripts/pi-sdk-conformance"))
    import package_slice as family
    import run_early_package_proof as schema
    import run_readme_examples as readme
    old_validate = schema.validate_paths

    def archive_paths(repo, scratch, cache):
        old_validate(repository, scratch, cache)
        if repo.resolve() != source or scratch.is_relative_to(source):
            raise ValueError("sealed source/scratch boundary")

    # Runtime adapters only: the exported source and package-producing harness bytes stay unchanged.
    schema.validate_paths = archive_paths
    schema.REPO = source
    family.REPO = source
    readme_adapter = configure_readme_adapter(readme, source, out)

    def local_run(command, **kwargs):
        if "env" in kwargs:
            kwargs["env"] = local_peer_environment(kwargs["env"])
        return guard.run(command, **kwargs)

    subprocess_facade = types.SimpleNamespace(
        run=local_run, TimeoutExpired=subprocess.TimeoutExpired,
        SubprocessError=subprocess.SubprocessError, CalledProcessError=subprocess.CalledProcessError)
    family.subprocess = subprocess_facade
    schema.subprocess = subprocess_facade
    readme.subprocess = subprocess_facade
    if args.prime_registry:
        run(["cargo", "fetch", "--locked"], source)
        prime = out / "registry-prime"
        prime.mkdir()
        (prime / "src").mkdir()
        (prime / "src/lib.rs").write_text("")
        manifest = '[package]\nname="phase20-proof-registry-prime"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[dependencies]\nflate2={version="=1.1.10",features=["zlib"]}\nregress={version="=0.12.0",features=["prohibit-unsafe"]}\nreqwest={version="=0.12.28",features=["gzip"],default-features=false}\nrustls={version="=0.23.40",features=["ring","aws_lc_rs"],default-features=false}\nlibz-sys="=1.1.30"\n'
        frozen = {"aws-lc-rs", "aws-lc-sys", "cc", "url", "idna", "idna_adapter", "tokio", "tokio-util", "hyper", "hyper-util", "http", "h2", "serde", "serde_core", "serde_derive", "serde_json", "time", "time-core", "time-macros", "proc-macro2", "quote", "syn"}
        packages = tomllib.loads((source / "Cargo.lock").read_text(encoding="utf-8"))["package"]
        frozen.update(item["name"] for item in packages if item["name"].startswith(("icu_", "futures-")))
        for name in sorted(frozen):
            versions = {item["version"] for item in packages if item["name"] == name and item.get("source", "").startswith("registry+")}
            if len(versions) == 1:
                manifest += name + '="=' + next(iter(versions)) + '"\n'
        (prime / "Cargo.toml").write_text(manifest, encoding="utf-8")
        run(["cargo", "generate-lockfile"], prime)
        run(["cargo", "fetch", "--locked"], prime)
    report = {"head": resolved_head, "source_archive_sha256": sha256(archive), "host": sys.platform,
              "target_cache": str(target), "families": [],
              "test_environment": local_peer_environment_receipt(os.environ),
              "producer_sha256": {path.name: sha256(path) for path in
                                  [pathlib.Path(__file__), pathlib.Path(__file__).with_name("full_sdk_matrix.py")]}}
    inputs = sealed_source_inputs(repository, source, resolved_head)
    inputs_path = out / "source-inputs.json"
    inputs_path.write_text(json.dumps(inputs, indent=2) + "\n", encoding="utf-8")
    report["source_inputs"] = {"file": str(inputs_path), "sha256": sha256(inputs_path),
                               "count": len(inputs), "commit": resolved_head}
    report["readme_adapter"] = readme_adapter
    try:
        for name in ["ws", "http", "crypto"]:
            result, evidence = run_family_with_snapshots(
                family, name, out / ("package-" + name), target)
            if result or evidence is None:
                report["failed_family"] = name
                raise ValueError(f"actual {name} package consumer did not pass: exit {result}")
            report["families"].append(evidence)
        package = out / "package-ws/package/opi-ai-0.8.3"
        full = guard.run([
            sys.executable, str(pathlib.Path(__file__).with_name("full_sdk_matrix.py")), "--package", str(package),
            "--target-dir", str(target), "--out", str(out / "full-sdk"), "--source", str(source), "--repository", str(repository)
        ], cwd=repository, timeout=21600)
        full_path = out / "full-sdk/evidence.json"
        report["full_sdk"] = json.loads(full_path.read_text(encoding="utf-8"))
        report["full_sdk_evidence"] = {"file": str(full_path), "sha256": sha256(full_path), "exit": full.returncode}
        require_final_sdk(report["full_sdk"], full.returncode)
        sys.argv = ["run_early_package_proof.py", "--scratch", str(out / "schema-package"), "--target-dir", str(target)]
        schema_exit = schema.main()
        schema_path = out / "schema-package/evidence.json"
        schema_evidence = json.loads(schema_path.read_text(encoding="utf-8"))
        report["schema"] = {"file": str(schema_path), "sha256": sha256(schema_path), "exit": schema_exit}
        if schema_exit or schema_evidence.get("verdict") != "PASS":
            raise ValueError("actual schema package consumer did not pass")
        sys.argv = ["run_readme_examples.py", "--offline", "--scratch", str(out / "readme-package"), "--target-dir", str(target)]
        readme_exit = readme.main()
        readme_path = out / "readme-package/evidence.json"
        readme_evidence = json.loads(readme_path.read_text(encoding="utf-8"))
        report["readme_examples"] = {"file": str(readme_path), "sha256": sha256(readme_path), "exit": readme_exit,
                                    "fixed_output": str(readme.HARNESS / "evidence/readme-examples-proof.json")}
        if readme_exit or readme_evidence.get("verdict") != "PASS":
            raise ValueError("actual same-source onboarding did not pass")
        readme.check_report(readme_evidence.get("execution"))
        if sealed_source_inputs(repository, source, resolved_head) != inputs:
            raise ValueError("sealed source changed during native verification")
        report["source_inputs"]["unchanged_after_execution"] = True
        report["verdict"] = "PASS"
    except Exception as error:
        report["error"] = str(error)
        report["verdict"] = "FAIL"
    (out / "native-proof.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
