"""Verification-branch entry point; consumes an immutable production commit."""
import argparse
import difflib
import hashlib
import json
import os
import pathlib
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib
import types

from full_sdk_matrix import ProcessGuard


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def retain_file(path, destination, expected):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(path, destination)
    observed = sha256(destination)
    if observed != expected:
        raise ValueError(f"retained bytes changed: {path}")
    return {"source": str(path), "file": str(destination), "sha256": observed}


# External diagnostic recipe: 040c6923b62adc01717da90cbda5ede78afcbc7c7ee4a7e632df608e32bddc47
ORIGINAL_SHA256 = "3e421f853836755d4d7cc46b69c8bec190944b807dc5d647f0b9a6cddd13d5d4"
EDITS = [
    ("HttpClient::new().redirect_policy() == RedirectPolicy::Limited(10)",
     "HttpClient::new().redirect_policy() == RedirectPolicy::Limited(20)"),
    ('"HttpClient default is Limited(10) and configurable"',
     '"HttpClient default is Limited(20) and configurable"'),
    ('matches!(&error, ProviderError::Network(summary) if summary.as_str() == "too many redirects")',
     'matches!(&error, ProviderError::StreamFailure(failure) if matches!(failure.cause(), ProviderError::Network(summary) if summary.as_str() == "too many redirects"))'),
    ('matches!(acquisition, ProviderError::Network(_))',
     'matches!(&acquisition, ProviderError::StreamFailure(failure) if matches!(failure.cause(), ProviderError::Network(_)))'),
]


def apply_http_diagnostic(source_harness: Path, destination: Path) -> dict:
    """Copy only consumer-http into a fresh external HARNESS root, then change four assertions/texts."""
    source_harness, destination = source_harness.resolve(), destination.resolve()
    if destination.exists() or destination.is_relative_to(source_harness):
        raise ValueError("diagnostic destination must be fresh and outside source harness")
    original = source_harness / "consumer-http/src/main.rs"
    before = original.read_bytes()
    if hashlib.sha256(before).hexdigest() != ORIGINAL_SHA256:
        raise ValueError("unreviewed original HTTP consumer source")
    text = before.decode("utf-8")
    for old, new in EDITS:
        if text.count(old) != 1:
            raise ValueError("diagnostic edit does not occur exactly once: " + old)
        text = text.replace(old, new)
    destination.mkdir(parents=True)
    shutil.copytree(source_harness / "consumer-http", destination / "consumer-http",
                    ignore=shutil.ignore_patterns("target", "Cargo.lock"))
    after = text.encode("utf-8")
    (destination / "consumer-http/src/main.rs").write_bytes(after)
    patch = "".join(difflib.unified_diff(before.decode("utf-8").splitlines(True), text.splitlines(True),
                                       fromfile="original/consumer-http/src/main.rs",
                                       tofile="diagnostic/consumer-http/src/main.rs"))
    (destination / "diagnostic.patch").write_text(patch, encoding="utf-8", newline="\n")
    result = {"scope": "diagnostic control only, not original-consumer acceptance", "original_sha256": ORIGINAL_SHA256,
              "diagnostic_sha256": hashlib.sha256(after).hexdigest(), "edits": EDITS,
              "patch_sha256": hashlib.sha256((destination / "diagnostic.patch").read_bytes()).hexdigest()}
    (destination / "diagnostic.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    if original.read_bytes() != before:
        raise ValueError("original source changed")
    return result


def is_known_http_baseline(report: dict) -> bool:
    """Require the complete original six-check report and these exact three stale assertions."""
    if not isinstance(report, dict):
        return False
    if report.get("verdict") != "FAIL" or report.get("profile") not in {"ordinary", "caller-union"}:
        return False
    if report.get("total") != 6 or report.get("failed") != 3:
        return False
    expected = [
        ("redirect-policy-owned-default", False, r"HttpClient default is Limited\(10\) and configurable"),
        ("same-origin-redirect-preserves-authorization", True, r'sends=2 hop2_target=Some\("/final"\)'),
        ("cross-origin-redirect-strips-authorization", True, r'origin_auth=Some\("Bearer pkg-consumer-key"\) target_auth=None target_sends=1'),
        ("redirect-limit-typed-failure-exact-sends", False, r'error=StreamFailure\(Network\(ProviderErrorSummary\("too many redirects"\)\)\) sends=3'),
        ("proxy-routing-with-scoped-bypass", True, r'proxy_hits=1 absolute_target=Some\("http://127\.0\.0\.1:[0-9]+/v1/chat/completions"\) target_sends=2'),
        ("acquisition-failure-and-single-terminal-eof", False, r'acquisition=StreamFailure\(Network\(ProviderErrorSummary\("error sending request for url \(http://127\.0\.0\.1:[0-9]+/v1/chat/completions\)"\)\)\) eof=StreamFailure\(StreamError\(ProviderErrorSummary\("stream ended without a terminal event"\)\)\) sends=1'),
    ]
    checks = report.get("checks", [])
    return isinstance(checks, list) and len(checks) == len(expected) and all(
        isinstance(item, dict) and isinstance(item.get("detail"), str) and item.get("name") == name and item.get("passed") is passed
        and re.fullmatch(detail, item.get("detail", "")) is not None
        for item, (name, passed, detail) in zip(checks, expected))


def run_family_with_snapshots(family, name, scratch, target, allow_known_http_failure=False):
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
        index = len(snapshots)
        if actual_config is not config or index >= len(config["profiles"]):
            raise ValueError("unexpected family/profile report order")
        profile = config["profiles"][index]["name"]
        snapshots.append(retain_profile(profile))
        (scratch / "retained-profile-artifacts.json").write_text(
            json.dumps({"profiles": snapshots, "complete": False}, indent=2), encoding="utf-8")

    original_subprocess = family.subprocess if allow_known_http_failure else None
    original_red = []

    def run_and_retain_red(command, **kwargs):
        result = original_subprocess.run(command, **kwargs)
        if name == "http" and result.returncode == 1 and not original_red:
            try:
                report = json.loads(result.stdout)
            except (TypeError, ValueError):
                return result
            if is_known_http_baseline(report) and report["profile"] == "ordinary":
                retained = retain_profile("ordinary")
                if (list(map(str, command)) != [retained["executable"]["source"]]
                        or pathlib.Path(kwargs["cwd"]).resolve() != (scratch / "consumer").resolve()):
                    raise ValueError("known HTTP report not emitted by the exact consumer command")
                original_red.append(retained)
        return result

    family.check_family_report = check_and_retain
    if allow_known_http_failure:
        family.subprocess = types.SimpleNamespace(run=run_and_retain_red,
                                                  TimeoutExpired=subprocess.TimeoutExpired)
    try:
        sys.argv = ["package_slice.py", "--family", name, "--offline", "--scratch", str(scratch),
                    "--target-dir", str(target)]
        result = family.main()
    finally:
        family.check_family_report = original_check
        if allow_known_http_failure:
            family.subprocess = original_subprocess
    if result:
        if allow_known_http_failure and original_red:
            return result, retain_known_http_failure(scratch, original_red)
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


def retain_known_http_failure(scratch, snapshots):
    """Bind the preserved red only after original family and lease accounting finish."""
    evidence_path = scratch / "evidence.json"
    evidence = json.loads(evidence_path.read_bytes())
    raw = scratch / "ordinary-consumer.stdout"
    report = json.loads(raw.read_bytes())
    steps = evidence["steps"]
    expected_steps = ["rustc-version", "cargo-version", "lease-start", "package", "consumer-lock",
                      "ordinary-metadata", "ordinary-build", "ordinary-consumer", "lease-end"]
    expected_exits = [0, 0, 0, 0, 0, 0, 0, 1, 0]
    inputs = {path.replace("\\", "/"): digest for path, digest in evidence["inputs"].items()}
    if (evidence.get("family") != "http" or evidence.get("verdict") != "FAIL"
            or evidence.get("profiles") != []
            or evidence.get("error") != "ordinary-consumer failed with exit 1; see scratch logs"
            or [step["name"] for step in steps] != expected_steps
            or [step["exit"] for step in steps] != expected_exits
            or inputs.get("consumer-http/src/main.rs") != ORIGINAL_SHA256
            or report.get("profile") != "ordinary" or not is_known_http_baseline(report)
            or len(snapshots) != 1):
        raise ValueError("HTTP failure exceeds the reviewed baseline classification")
    snapshot_path = scratch / "retained-profile-artifacts.json"
    snapshot_path.write_text(json.dumps({"profiles": snapshots, "complete": False,
        "scope": "original ordinary profile failed; retained before any diagnostic build"}, indent=2), encoding="utf-8")
    return {"family": "http", "verdict": "FAIL", "known_baseline": "B20",
            "scope": "original consumer remains failing; not original-consumer acceptance",
            "evidence": str(evidence_path), "sha256": sha256(evidence_path),
            "raw_report": str(raw), "raw_report_sha256": sha256(raw),
            "retained_artifacts": str(snapshot_path), "retained_artifacts_sha256": sha256(snapshot_path)}


def aggregate_verdict(full_verdict, known_baselines):
    if full_verdict not in {"PASS", "COMPLETE-WITH-KNOWN-BASELINE-FAILURES"}:
        return "FAIL-UNCLASSIFIED"
    return "COMPLETE-WITH-KNOWN-BASELINE-FAILURES" if known_baselines or full_verdict != "PASS" else "PASS"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--repository", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--head", default="808ac5ac1691a3e5580a3c75bd20b6c75b408efe")
    parser.add_argument("--prime-registry", action="store_true")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.head):
        parser.error("--head must be an exact full commit SHA")
    repository = pathlib.Path(args.repository).resolve()
    resolved_head = subprocess.check_output(["git", "rev-parse", args.head + "^{commit}"], cwd=repository, text=True).strip()
    if resolved_head != args.head:
        raise ValueError("production source commit did not resolve exactly")
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
                      AWS_LC_SYS_USE_SYSTEM="0", LIBZ_SYS_STATIC="1", VCPKGRS_DISABLE="1", CC_ENABLE_DEBUG_OUTPUT="1")
    sys.path.insert(0, str(source / "scripts/pi-sdk-conformance"))
    import package_slice as family
    import run_early_package_proof as schema
    old_validate = schema.validate_paths

    def archive_paths(repo, scratch, cache):
        old_validate(repository, scratch, cache)
        if repo.resolve() != source or scratch.is_relative_to(source):
            raise ValueError("sealed source/scratch boundary")

    # Runtime adapters only: the exported source and package-producing harness bytes stay unchanged.
    schema.validate_paths = archive_paths
    schema.REPO = source
    family.REPO = source
    subprocess_facade = types.SimpleNamespace(run=guard.run, TimeoutExpired=subprocess.TimeoutExpired)
    family.subprocess = subprocess_facade
    schema.subprocess = subprocess_facade
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
              "target_cache": str(target), "families": [], "known_baseline_failures": [],
              "producer_sha256": {path.name: sha256(path) for path in
                                  [pathlib.Path(__file__), pathlib.Path(__file__).with_name("full_sdk_matrix.py")]}}
    for name in ["ws", "http", "crypto"]:
        result, evidence = run_family_with_snapshots(
            family, name, out / ("package-" + name), target, allow_known_http_failure=name == "http")
        if result:
            if name != "http" or not evidence or evidence.get("known_baseline") != "B20":
                report["failed_family"] = name
                report["verdict"] = "FAIL-UNCLASSIFIED"
                (out / "native-proof.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
                return 1
            report["families"].append(evidence)
            report["known_baseline_failures"].append({"batch": "B20", "family": "http",
                "original": evidence, "scope": "three original harness assertions conflict with current SDK contracts"})
            original_harness = family.HARNESS
            diagnostic_harness = out / "http-diagnostic-harness"
            recipe = apply_http_diagnostic(original_harness, diagnostic_harness)
            recipe["file"] = str(diagnostic_harness / "diagnostic.json")
            recipe["sha256"] = sha256(diagnostic_harness / "diagnostic.json")
            try:
                family.HARNESS = diagnostic_harness
                diagnostic_result, diagnostic = run_family_with_snapshots(
                    family, "http", out / "package-http-diagnostic", target)
            finally:
                family.HARNESS = original_harness
            if diagnostic_result:
                report["failed_family"] = "http-diagnostic"
                report["verdict"] = "FAIL-UNCLASSIFIED"
                (out / "native-proof.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
                return 1
            diagnostic.update(scope="external diagnostic consumer only; original HTTP harness remains FAIL", recipe=recipe)
            report["http_diagnostic"] = diagnostic
        else:
            report["families"].append(evidence)
    package = out / "package-ws/package/opi-ai-0.8.3"
    full = guard.run([
        sys.executable, str(pathlib.Path(__file__).with_name("full_sdk_matrix.py")), "--package", str(package),
        "--target-dir", str(target), "--out", str(out / "full-sdk"), "--source", str(source), "--repository", str(repository)
    ], cwd=repository, timeout=21600)
    report["full_sdk"] = json.loads((out / "full-sdk/evidence.json").read_text(encoding="utf-8"))
    expected_exit = {"PASS": 0, "COMPLETE-WITH-KNOWN-BASELINE-FAILURES": 2}.get(report["full_sdk"].get("verdict"))
    if full.returncode != expected_exit:
        report["full_sdk_failure"] = full.returncode
        report["full_sdk"]["verdict"] = "FAIL-UNCLASSIFIED"
    sys.argv = ["run_early_package_proof.py", "--scratch", str(out / "schema-package"), "--target-dir", str(target)]
    if schema.main():
        return 1
    report["verdict"] = aggregate_verdict(report["full_sdk"]["verdict"], report["known_baseline_failures"])
    (out / "native-proof.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["verdict"] == "PASS" else 2 if report["verdict"] == "COMPLETE-WITH-KNOWN-BASELINE-FAILURES" else 1


if __name__ == "__main__":
    raise SystemExit(main())
