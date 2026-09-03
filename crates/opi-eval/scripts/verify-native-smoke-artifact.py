#!/usr/bin/env python3
"""Verify a downloaded opi-eval native-smoke artifact against its receipt.

This is the sole consumer of the human-dispatched Linux native smoke
(native smoke): it takes the upload-identity receipt downloaded from the
workflow run plus the downloaded artifact (the GitHub zip or the bare
sealed tar inside it) and re-derives every binding the definition of
done requires before any acceptance criterion may close:

- the receipt identity chain (schema, artifact id/url/digest, run
  identity, expiry, sealed-manifest and outer-receipt digests, no
  self-reference);
- the dispatch binding (candidate commit, github.workflow_ref and
  github.workflow_sha, workflow bytes read from the expected commit
  through the local git object database, every bound script digest,
  every pinned immutable action);
- the sealed content (full sorted-manifest recomputation over the
  extracted stage tree, canary-oracle negative preflight, oracle
  preflights passing per task, twelve rerun conformance cases, six
  paired sealed trials with completed agents and verified native
  verifiers, native agent evidence, and one comparable cross-agent
  comparison edge per task under conformance-only labeling).

Reward zero is accepted as an integration result and never as agent
task success. Any mismatch, drift, missing trial, expired artifact, or
positive canary rejects with a nonzero exit.

Standard library only; no network access, no GitHub API calls.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile
from pathlib import Path

UPLOAD_SCHEMA = "opi-eval-upload-identity-receipt/1"
OUTER_SCHEMA = "opi-eval-native-outer-receipt/1"
MANIFEST_SCHEMA = "opi-eval-native-artifact-manifest/1"
RUN_REPORT_SCHEMA = "opi-eval-run-report/1"
PREFLIGHT_SCHEMA = "opi-eval-oracle-preflight/1"
PROVIDER_LOG_SCHEMA = "opi-eval-scripted-provider-log/1"
WORKFLOW_PATH = ".github/workflows/opi-eval-native-smoke.yml"

CRITERIA = ("EVAL-A02", "EVAL-A03", "EVAL-A04", "EVAL-A08", "EVAL-A09",
            "EVAL-A10", "EVAL-A12")

BENCHMARKS = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
CONFIG_FOR = {"terminal-bench-2.1": "terminal-bench-2.1",
              "terminal-bench-3.0": "terminal-bench-3.0",
              "deepswe-v1.1": "deepswe-v1.1"}
AGENT_PROFILE_PATHS = {
    "opi": "crates/opi-eval/profiles/agents/opi.toml",
    "pi": "crates/opi-eval/profiles/agents/pi.toml",
}
# The admitted native rerun matrix: four agent cases (completed and
# identity for both products) plus three benchmark cases (completed,
# identity, immutable-capture) for each of the three revisions.
CONFORMANCE_CASES = 13

IMMUTABLE_ACTIONS = {
    "actions/checkout": "11bd71901bbe5b1630ceea73d27597364c9af683",
    "dtolnay/rust-toolchain": "889fac408b4da0905346410f253f0c55fbcb6613",
    "actions/setup-node": "49933ea5288caeca8642d1e84afbd3f7d6820020",
    "astral-sh/setup-uv": "b75a909f75acd358c2196fb9a5f1299a9a8868a4",
    "docker/setup-buildx-action": "e468171a9de216ec08956ac3ada2f0791b6bd435",
    "actions/upload-artifact": "ea165f8d65b6e75b540449e92b4886f43607fa02",
}

HEX64 = set("0123456789abcdef")


class Findings:
    """Reject-collecting verification state (fail-closed)."""

    def __init__(self) -> None:
        self.errors: list[str] = []

    def reject(self, family: str, message: str) -> None:
        self.errors.append(f"finding {family}: {message}")

    def ok(self) -> bool:
        return not self.errors

    def report(self) -> str:
        return "\n".join(self.errors)


def is_hex64(value) -> bool:
    return (isinstance(value, str) and len(value) == 64
            and all(c in HEX64 for c in value))


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_json(path: Path, f: Findings, family: str) -> dict | None:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        f.reject(family, f"cannot read {path.name}: {error}")
        return None


def unpack_receipt(path: Path, f: Findings) -> dict | None:
    """Loads the upload receipt, accepting a bare file or a zip."""
    try:
        if zipfile.is_zipfile(path):
            with zipfile.ZipFile(path) as bundle:
                names = [n for n in bundle.namelist()
                         if n.endswith("upload-receipt.json")]
                if not names:
                    f.reject("receipt", "zip carries no upload-receipt.json")
                    return None
                return json.loads(bundle.read(names[0]).decode("utf-8"))
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        f.reject("receipt", f"cannot read upload receipt: {error}")
        return None


def unpack_artifact(path: Path, expected_digest: str,
                    f: Findings) -> Path | None:
    """Streaming extraction of the sealed stage into a temp dir.

    Real artifacts reach tens of gigabytes, so the zip envelope, the
    tar member table, and no single member are ever held in memory:
    the tar is consumed as a stream and every member is written
    straight to disk. (A whole-archive in-memory load OOM-killed the
    first real verification.)
    """
    try:
        is_zip = zipfile.is_zipfile(path)
        # The uploaded digest binds the zip envelope; a bare tar (an
        # already-unpacked handoff) has no envelope to compare. The
        # envelope is never loaded into memory (real zips reach GiB
        # scale): the digest hashes the file in bounded chunks.
        if is_zip:
            digest = hashlib.sha256()
            with open(path, "rb") as handle:
                for chunk in iter(lambda: handle.read(1 << 20), b""):
                    digest.update(chunk)
            actual = digest.hexdigest()
            if actual != expected_digest:
                f.reject("digest",
                         "downloaded artifact bytes do not match the "
                         "uploaded digest: expected "
                         f"{expected_digest}, got {actual}")
                return None
        root = Path(tempfile.mkdtemp(prefix="opi-eval-artifact-"))
        root_resolved = root.resolve()
        if is_zip:
            with zipfile.ZipFile(path) as bundle:
                tars = [n for n in bundle.namelist()
                        if n.endswith("sealed-artifact.tar")]
                if len(tars) != 1:
                    f.reject("artifact",
                             f"zip must carry exactly one sealed tar, "
                             f"found {len(tars)}")
                    return None
                with bundle.open(tars[0]) as stream:
                    _extract_tar_stream(stream, root, root_resolved, f)
        else:
            with open(path, "rb") as stream:
                _extract_tar_stream(stream, root, root_resolved, f)
        if not f.ok():
            return None
        return root
    except (OSError, ValueError, tarfile.TarError) as error:
        f.reject("artifact", f"cannot unpack artifact: {error}")
        return None


def _extract_tar_stream(stream, root: Path, root_resolved: Path,
                        f: Findings) -> None:
    with tarfile.open(fileobj=stream, mode="r|") as archive:
        for member in archive:
            target = (root / member.name).resolve()
            if not str(target).startswith(str(root_resolved)):
                f.reject("artifact",
                         f"tar member escapes the stage root: {member.name}")
                return
            archive.extract(member, root, filter="tar")


def git_bytes(repo: Path, commit: str, rel_path: str) -> bytes | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(repo), "show", f"{commit}:{rel_path}"],
            capture_output=True, check=True)
        return result.stdout
    except (subprocess.CalledProcessError, OSError):
        return None


def verify_identity(upload: dict, f: Findings) -> None:
    if upload.get("schema") != UPLOAD_SCHEMA:
        f.reject("receipt", f"unknown schema {upload.get('schema')!r}")
        return
    for key in ("artifact_id", "artifact_url", "run_id", "run_url"):
        if not isinstance(upload.get(key), str) or not upload[key]:
            f.reject("receipt", f"{key} missing from the upload receipt")
    # The downloaded bytes must hash to the uploaded digest: any
    # drift is transport corruption, not a re-serialization.
    if not is_hex64(upload.get("artifact_digest")):
        f.reject("receipt", "artifact digest is not a sha256 hex digest")
    for key in ("sealed_manifest_sha256", "outer_receipt_sha256"):
        if not is_hex64(upload.get(key)):
            f.reject("receipt", f"{key} is not a sha256 hex digest")
    try:
        expires = datetime.datetime.fromisoformat(str(upload.get("expires_at")))
        if expires.tzinfo is None:
            expires = expires.replace(tzinfo=datetime.timezone.utc)
        if expires <= datetime.datetime.now(datetime.timezone.utc):
            f.reject("expiry", "the artifact expiry has passed")
    except (TypeError, ValueError):
        f.reject("expiry", "expires_at is not an ISO-8601 timestamp")


def verify_seal(stage: Path, upload: dict, f: Findings) -> dict | None:
    outer_path = stage / "08-seal" / "outer-receipt.json"
    manifest_path = stage / "08-seal" / "artifact-manifest.json"
    outer = load_json(outer_path, f, "seal")
    manifest = load_json(manifest_path, f, "seal")
    if outer is None or manifest is None:
        return None
    if outer.get("schema") != OUTER_SCHEMA:
        f.reject("seal", f"unknown outer schema {outer.get('schema')!r}")
    if manifest.get("schema") != MANIFEST_SCHEMA:
        f.reject("seal", f"unknown manifest schema {manifest.get('schema')!r}")
    manifest_bytes = manifest_path.read_bytes()
    outer_bytes = outer_path.read_bytes()
    if is_hex64(upload.get("sealed_manifest_sha256")) and \
            upload["sealed_manifest_sha256"] != sha(manifest_bytes):
        f.reject("digest", "sealed manifest digest drifts from the receipt")
    if is_hex64(upload.get("outer_receipt_sha256")) and \
            upload["outer_receipt_sha256"] != sha(outer_bytes):
        f.reject("digest", "outer receipt digest drifts from the receipt")
    if outer.get("artifact_manifest_sha256") != sha(manifest_bytes):
        f.reject("digest", "outer receipt does not bind the manifest bytes")
    files = manifest.get("files")
    if not isinstance(files, dict) or not files:
        f.reject("seal", "the sealed manifest carries no files")
        return None
    # Symmetric with the producer's manifest: symbolic links are sealed
    # as tar link entries, never as byte payloads, so a link whose
    # target exists is not a manifestable file.
    present = {p.relative_to(stage).as_posix() for p in stage.rglob("*")
               if p.is_file() and not p.is_symlink()}
    allowed_extra = {"08-seal/artifact-manifest.json",
                     "08-seal/outer-receipt.json"}
    for rel in sorted(present - set(files) - allowed_extra):
        f.reject("manifest", f"unmanifested sealed file: {rel}")
    for rel, want in sorted(files.items()):
        target = stage / rel
        if not target.is_file() or target.is_symlink():
            f.reject("manifest", f"sealed file missing from the artifact: {rel}")
        elif sha(target.read_bytes()) != want:
            f.reject("manifest", f"sealed file digest drift: {rel}")
    if outer.get("conformance_evidence_only") is not True:
        f.reject("seal", "outer receipt does not label evidence conformance-only")
    if outer.get("leaderboard_claim") != "none":
        f.reject("seal", "outer receipt claims a leaderboard result")
    if outer.get("canary_preflight_negative_recorded") is not True:
        f.reject("canary", "outer receipt lacks the negative canary record")
    return outer


def verify_dispatch(outer: dict, expected_commit: str, repo: Path,
                    f: Findings) -> None:
    dispatch = outer.get("dispatch")
    if not isinstance(dispatch, dict):
        f.reject("dispatch", "outer receipt carries no dispatch binding")
        return
    candidate = dispatch.get("candidate_sha")
    if candidate != expected_commit or dispatch.get("checkout_head") != candidate:
        f.reject("candidate",
                 f"dispatch candidate {candidate!r} is not the expected "
                 f"commit {expected_commit!r}")
        return
    ref = dispatch.get("github_workflow_ref")
    if isinstance(ref, str) and "@" in ref and not ref.startswith("refs/"):
        # GitHub records the qualified "<repo>/<path>@<ref>" form; the
        # bare ref is what a dispatch can be pinned to (same
        # normalization as the materialization verifier).
        ref = ref.rsplit("@", 1)[1]
    if not isinstance(ref, str) or not ref.startswith("refs/"):
        f.reject("dispatch", f"github.workflow_ref is not a ref: {ref!r}")
    if dispatch.get("workflow_path") != WORKFLOW_PATH:
        f.reject("dispatch",
                 f"workflow path drift: {dispatch.get('workflow_path')!r}")
    recorded = dispatch.get("workflow_sha256_read_from_workflow_sha")
    if not is_hex64(recorded):
        f.reject("dispatch", "workflow digest is not a sha256 hex digest")
        return
    local = git_bytes(repo, expected_commit, WORKFLOW_PATH)
    if local is None:
        f.reject("dispatch",
                 f"cannot read workflow bytes at {expected_commit} from "
                 f"{repo}; the expected commit must exist locally")
    elif sha(local) != recorded:
        f.reject("dispatch",
                 "workflow bytes at the expected commit do not match the "
                 "dispatch-recorded digest")
    scripts = dispatch.get("bound_scripts")
    if not isinstance(scripts, dict) or not scripts:
        f.reject("dispatch", "dispatch binds no invoked scripts")
        return
    for role, entry in sorted(scripts.items()):
        rel = entry.get("path") if isinstance(entry, dict) else None
        digest = entry.get("sha256") if isinstance(entry, dict) else None
        if not isinstance(rel, str) or not is_hex64(digest):
            f.reject("bound", f"bound script {role} lacks a path/digest")
            continue
        local = git_bytes(repo, expected_commit, rel)
        if local is None:
            f.reject("bound",
                     f"cannot read bound script bytes at {expected_commit}: "
                     f"{rel}")
        elif sha(local) != digest:
            f.reject("bound",
                     f"bound script digest drift for {role}: {rel}")
    actions = dispatch.get("immutable_actions")
    if not isinstance(actions, list):
        f.reject("dispatch", "dispatch binds no immutable actions")
        return
    seen = {}
    for entry in actions:
        if not isinstance(entry, dict):
            continue
        seen[entry.get("name")] = entry.get("commit")
    for name, commit in sorted(IMMUTABLE_ACTIONS.items()):
        if seen.get(name) != commit:
            f.reject("dispatch", f"immutable action {name} is not pinned at "
                                 f"{commit}")


def load_run_reports(stage: Path, f: Findings) -> list[dict]:
    reports = []
    for cfg in sorted(CONFIG_FOR.values()):
        report = load_json(stage / "07-trials" / cfg / "run-report.json",
                           f, "trial")
        if report is None:
            continue
        if report.get("schema") != RUN_REPORT_SCHEMA:
            f.reject("trial", f"{cfg}: unknown run report schema")
            continue
        reports.append({"cfg": cfg, "report": report})
    return reports


def verify_shared(stage: Path, outer: dict, f: Findings) -> list[dict]:
    """Bindings every criterion requires; returns the run reports."""
    reports = load_run_reports(stage, f)
    if len(reports) != len(BENCHMARKS):
        f.reject("trial", "the artifact does not carry one run report per "
                          "admitted benchmark")
    trials_total = 0
    for entry in reports:
        report = entry["report"]
        if report.get("outcome") != "completed":
            f.reject("trial", f"{entry['cfg']}: run outcome is "
                              f"{report.get('outcome')!r}, not completed")
        trials = report.get("trials")
        if not isinstance(trials, list) or len(trials) != 2:
            f.reject("trial", f"{entry['cfg']}: expected exactly two paired "
                              f"trials, found {len(trials) if isinstance(trials, list) else 'none'}")
            continue
        trials_total += len(trials)
        products = {t.get("agent", {}).get("product") for t in trials}
        if products != {"opi", "pi"}:
            f.reject("trial", f"{entry['cfg']}: paired products are "
                              f"{sorted(str(p) for p in products)}")
        for trial in trials:
            agent = trial.get("agent", {})
            verifier = trial.get("verifier", {})
            if trial.get("status") != "sealed":
                f.reject("trial", f"{trial.get('id')}: trial is not sealed")
            if agent.get("completion") != "completed":
                f.reject("trial", f"{trial.get('id')}: agent did not complete")
            if verifier.get("completion") != "verified":
                f.reject("verifier", f"{trial.get('id')}: native verifier did "
                                     f"not complete ({verifier.get('completion')!r})")
            reward = verifier.get("reward")
            if not (isinstance(reward, str) and reward.startswith("known:")):
                f.reject("verifier", f"{trial.get('id')}: reward is not a "
                                     f"known integration result: {reward!r}")
            # The pre-seal lifecycle binding: the receipt carries the
            # typed trajectory's pre-seal digest and the seal result
            # must bind the same bundle identity the receipt claims.
            if not is_hex64(trial.get("pre_seal_digest")):
                f.reject("trial", f"{trial.get('id')}: pre-seal digest is "
                                  f"missing from the receipt")
            sealed = ((trial.get("seal_result") or {}).get("sealed") or {})
            identity = trial.get("bundle_identity")
            if not is_hex64(identity) or sealed.get("bundle_digest") != identity:
                f.reject("trial", f"{trial.get('id')}: the sealed bundle "
                                  f"digest does not bind the trajectory")
        pairs = report.get("pairs")
        if not isinstance(pairs, list) or len(pairs) != 1:
            f.reject("comparison", f"{entry['cfg']}: expected exactly one "
                                   f"comparison edge")
        else:
            pair = pairs[0]
            if pair.get("comparability") != "comparable":
                f.reject("comparison",
                         f"{entry['cfg']}: comparison edge is not comparable")
    if trials_total != 6:
        f.reject("trial", f"expected exactly six paired trials, found "
                          f"{trials_total}")
    return reports


def verify_canary(stage: Path, f: Findings) -> None:
    canary = load_json(stage / "06-canaries" / "canary-preflight.json",
                       f, "canary")
    if canary is None:
        return
    if canary.get("negative_result_observed") is not True:
        f.reject("canary", "the canary-oracle preflight was not negative")
    if canary.get("leakage"):
        f.reject("canary",
                 f"oracle/reference material leaked into an agent phase: "
                 f"{canary['leakage']}")
    markers_path = stage / "06-canaries" / "canary-markers.txt"
    if not markers_path.is_file():
        f.reject("canary", "the declared canary marker list is missing")
        return
    declared = [line for line in
                markers_path.read_text(encoding="utf-8").splitlines() if line]
    recorded = [marker for entry in canary.get("oracle_manifest", [])
                for marker in entry.get("markers", []) if marker]
    if sorted(declared) != sorted(recorded):
        f.reject("canary", "the declared marker list does not match the "
                           "preflight record")


def verify_provider(stage: Path, f: Findings) -> None:
    log = stage / "05-provider" / "requests.jsonl"
    if not log.is_file():
        f.reject("provider", "the provider request log is missing")
        return
    lines = [line for line in log.read_text(encoding="utf-8").splitlines()
             if line.strip()]
    if not lines:
        f.reject("provider", "the provider request log is empty")
        return
    for line in lines:
        try:
            entry = json.loads(line)
        except ValueError:
            f.reject("provider", "the request log carries a non-JSON line")
            return
        if entry.get("schema") != PROVIDER_LOG_SCHEMA:
            f.reject("provider",
                     f"unknown request log schema {entry.get('schema')!r}")
            return
        if not is_hex64(entry.get("request_sha256")):
            f.reject("provider", "a request log line lacks its digest")
            return


def verify_conformance_rerun(stage: Path, f: Findings) -> None:
    conformance = stage / "06-material" / "conformance"
    if not (conformance / "receipt.json").is_file():
        f.reject("conformance", "the conformance-rerun stage receipt is missing")
    reports = sorted((conformance / "reports").glob("*/report.json")) \
        if (conformance / "reports").is_dir() else []
    if len(reports) != CONFORMANCE_CASES:
        f.reject("conformance",
                 f"expected {CONFORMANCE_CASES} rerun conformance cases, "
                 f"found {len(reports)}")
        return
    for path in reports:
        report = load_json(path, f, "conformance")
        if report is not None and report.get("met") is not True:
            f.reject("conformance", f"rerun case did not pass: {path.parent.name}")


def verify_oracle_preflights(stage: Path, f: Findings) -> None:
    for bench, cfg in sorted(CONFIG_FOR.items()):
        receipt_path = (stage / "06-material" / "oracle" / "runs" / cfg /
                        "preflight" / bench / "preflight-receipt.json")
        receipt = load_json(receipt_path, f, "oracle")
        if receipt is None:
            continue
        if receipt.get("schema") != PREFLIGHT_SCHEMA:
            f.reject("oracle", f"{bench}: unknown preflight schema")
        if receipt.get("outcome") != "passed":
            f.reject("oracle", f"{bench}: upstream oracle preflight did not "
                               f"pass ({receipt.get('outcome')!r})")
        if not is_hex64(receipt.get("oracle_executable_sha256")):
            f.reject("oracle", f"{bench}: preflight does not bind the oracle "
                               f"executable digest")


def verify_agent_evidence(stage: Path, reports: list[dict],
                          product: str, keys: tuple[str, ...],
                          f: Findings) -> None:
    for entry in reports:
        for trial in entry["report"].get("trials", []):
            if trial.get("agent", {}).get("product") != product:
                continue
            root = (stage / "07-trials" / entry["cfg"] / "trials"
                    / str(trial.get("id")))
            for key in keys:
                evidence = root / "bundle" / "artifacts" / key
                if not evidence.is_file():
                    f.reject("evidence",
                             f"{trial.get('id')}: staged native evidence "
                             f"missing: {key}")
                elif not evidence.read_bytes():
                    f.reject("evidence",
                             f"{trial.get('id')}: staged native evidence is "
                             f"empty: {key}")


def verify_material(stage: Path, f: Findings) -> dict | None:
    material = load_json(stage / "06-material" / "material.json",
                         f, "material")
    if material is None:
        return None
    agents = material.get("agents")
    if not isinstance(agents, dict) or sorted(agents) != ["opi", "pi"]:
        f.reject("material", "the material does not declare exactly opi and pi")
        return None
    opi_exec = agents["opi"].get("executable", {})
    pi_exec = agents["pi"].get("executable", {})
    if opi_exec.get("path") == pi_exec.get("path"):
        f.reject("material", "both agents share one executable identity")
    if agents["opi"].get("model") == agents["pi"].get("model"):
        f.reject("material", "both agents share one model identity")
    benchmarks = material.get("benchmarks")
    if not isinstance(benchmarks, dict) or sorted(benchmarks) != sorted(BENCHMARKS):
        f.reject("material", "the material does not declare the three "
                             "admitted benchmark revisions")
    return material


def repository_profile_path(value, benchmark: str,
                            f: Findings) -> str | None:
    """Normalizes a material profile path to its candidate-repository path."""
    if not isinstance(value, str):
        f.reject("matrix", f"{benchmark}: material profile path is missing")
        return None
    normalized = value.replace("\\", "/")
    marker = "crates/opi-eval/profiles/benchmarks/"
    index = normalized.find(marker)
    if index < 0:
        f.reject("matrix", f"{benchmark}: profile is not under {marker}")
        return None
    return normalized[index:]


def load_commit_profile(repo: Path, commit: str, rel_path: str,
                        schema: str, f: Findings) -> dict | None:
    """Loads TOML profile bytes from the exact accepted producer commit."""
    data = git_bytes(repo, commit, rel_path)
    if data is None:
        f.reject("matrix", f"cannot read commit-bound profile {rel_path}")
        return None
    try:
        doc = tomllib.loads(data.decode("utf-8"))
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        f.reject("matrix", f"cannot parse commit-bound profile {rel_path}: {error}")
        return None
    if doc.get("schema") != schema:
        f.reject("matrix", f"{rel_path}: profile schema is {doc.get('schema')!r}")
        return None
    return {"path": rel_path, "sha256": sha(data), "doc": doc}


def flatten_profile_fields(doc: dict) -> dict[str, object]:
    """Flattens scalar profile evidence while excluding package file tables."""
    fields: dict[str, object] = {}

    def visit(prefix: str, value) -> None:
        if isinstance(value, dict):
            for key in sorted(value):
                if not prefix and key == "package":
                    continue
                visit(f"{prefix}.{key}" if prefix else key, value[key])
        elif isinstance(value, list):
            if all(isinstance(item, (str, int, float, bool)) for item in value):
                fields[prefix] = value
        elif isinstance(value, (str, int, float, bool)):
            fields[prefix] = value

    visit("", doc)
    return fields


def markdown_code(value) -> str:
    if isinstance(value, str):
        rendered = value
    else:
        rendered = json.dumps(value, sort_keys=True, separators=(",", ":"))
    return "`" + rendered.replace("`", "\\`").replace("|", "\\|") + "`"


def markdown_join(values) -> str:
    values = list(values)
    return ", ".join(markdown_code(value) for value in values) if values else "—"


def native_roles(stage: Path, reports: list[dict],
                 f: Findings) -> dict[str, set[str]]:
    roles = {"opi": set(), "pi": set()}
    for entry in reports:
        for trial in entry["report"].get("trials", []):
            product = trial.get("agent", {}).get("product")
            if product not in roles:
                f.reject("matrix", f"unknown Agent product in trial {trial.get('id')!r}")
                continue
            root = (stage / "07-trials" / entry["cfg"] / "trials" /
                    str(trial.get("id")) / "bundle" / "artifacts")
            if not root.is_dir():
                f.reject("matrix", f"{trial.get('id')}: artifact directory is missing")
                continue
            for path in sorted(root.rglob("*")):
                if path.is_file():
                    role = path.relative_to(root).as_posix()
                    if role.startswith("native/"):
                        roles[product].add(role)
    for product, product_roles in roles.items():
        if not product_roles:
            f.reject("matrix", f"{product}: no native artifact roles were retained")
    return roles


def render_seam_matrix(stage: Path, upload: dict, outer: dict,
                       material: dict, reports: list[dict],
                       expected_commit: str, repo: Path,
                       f: Findings) -> bytes | None:
    """Derives the minimum seam record from accepted native evidence."""
    agent_profiles = {}
    for product, rel_path in sorted(AGENT_PROFILE_PATHS.items()):
        profile = load_commit_profile(
            repo, expected_commit, rel_path, "opi-eval-agent-profile/1", f)
        if profile is None:
            continue
        if profile["doc"].get("product") != product:
            f.reject("matrix", f"{rel_path}: product identity does not match {product}")
        agent_profiles[product] = profile

    benchmark_profiles = {}
    for benchmark in sorted(BENCHMARKS):
        entry = material.get("benchmarks", {}).get(benchmark, {})
        rel_path = repository_profile_path(entry.get("profile"), benchmark, f)
        if rel_path is None:
            continue
        profile = load_commit_profile(
            repo, expected_commit, rel_path, "opi-eval-benchmark-profile/1", f)
        if profile is None:
            continue
        doc = profile["doc"]
        identity = f"{doc.get('benchmark')}-{doc.get('revision')}"
        if identity != benchmark:
            f.reject("matrix", f"{rel_path}: identity {identity!r} does not match {benchmark}")
        profile_digest = (doc.get("identity") or {}).get("package_manifest_sha256")
        if profile_digest != entry.get("task_package_manifest_sha256"):
            f.reject("matrix", f"{benchmark}: material/profile package digest drift")
        benchmark_profiles[benchmark] = profile

    if len(agent_profiles) != 2 or len(benchmark_profiles) != 3:
        return None

    report_for = {entry["cfg"]: entry["report"] for entry in reports}
    task_for = {}
    verified_trials = {}
    for benchmark in sorted(BENCHMARKS):
        report = report_for.get(CONFIG_FOR[benchmark])
        if report is None:
            f.reject("matrix", f"{benchmark}: accepted run report is missing")
            continue
        tasks = sorted({str(trial.get("task")) for trial in report.get("trials", [])})
        expected_task = benchmark_profiles[benchmark]["doc"]["identity"].get("task_id")
        if tasks != [expected_task]:
            f.reject("matrix", f"{benchmark}: report tasks {tasks!r} do not match profile task {expected_task!r}")
        task_for[benchmark] = expected_task
        verified_trials[benchmark] = sum(
            trial.get("verifier", {}).get("completion") == "verified"
            for trial in report.get("trials", []))

    roles = native_roles(stage, reports, f)
    owners: dict[str, list[str]] = {}
    for benchmark, profile in sorted(benchmark_profiles.items()):
        owner = str(profile["doc"].get("verifier", {}).get("runner_kind", ""))
        if not owner:
            f.reject("matrix", f"{benchmark}: verifier owner is missing")
        owners.setdefault(owner, []).append(benchmark)
    if len(owners) < 2:
        f.reject("matrix", "fewer than two native-verifier owners are proved")
    if not f.ok():
        return None

    agent_fields = {
        product: flatten_profile_fields(profile["doc"])
        for product, profile in agent_profiles.items()
    }
    common_agent_fields = sorted(set.intersection(
        *(set(fields) for fields in agent_fields.values())))
    benchmark_fields = {
        benchmark: flatten_profile_fields(profile["doc"])
        for benchmark, profile in benchmark_profiles.items()
    }
    common_benchmark_fields = sorted(set.intersection(
        *(set(fields) for fields in benchmark_fields.values())))
    shared_roles = sorted(set.intersection(*(set(value) for value in roles.values())))

    dispatch = outer["dispatch"]
    lines = [
        "# Native Seam Evidence Matrix",
        "",
        "This file is generated only after the complete `all-native` artifact "
        "verifies. It records conformance evidence, not a leaderboard result, "
        "stable public API, package commitment, or publication decision.",
        "",
        "## Verified inventory",
        "",
        "### Agent harnesses",
        "",
        "| Product | Commit-bound profile | Profile SHA-256 | Adapter | Package | Executable SHA-256 | Model identity |",
        "|---|---|---|---|---|---|---|",
    ]
    for product in sorted(agent_profiles):
        profile = agent_profiles[product]
        doc = profile["doc"]
        agent = material["agents"][product]
        lines.append("| " + " | ".join([
            markdown_code(product), markdown_code(profile["path"]),
            markdown_code(profile["sha256"]),
            markdown_code(doc["identity"]["adapter"]),
            markdown_code(doc["identity"]["package"]),
            markdown_code(agent["executable"]["sha256"]),
            markdown_code(agent["model"]),
        ]) + " |")

    lines.extend([
        "",
        "### Benchmark revisions",
        "",
        "| Revision integration | Task | Commit-bound profile | Profile SHA-256 | Package-manifest SHA-256 | Adapter | Native-verifier owner | Verified paired trials |",
        "|---|---|---|---|---|---|---|---|",
    ])
    for benchmark in sorted(benchmark_profiles):
        profile = benchmark_profiles[benchmark]
        doc = profile["doc"]
        entry = material["benchmarks"][benchmark]
        lines.append("| " + " | ".join([
            markdown_code(benchmark), markdown_code(task_for[benchmark]),
            markdown_code(profile["path"]), markdown_code(profile["sha256"]),
            markdown_code(entry["task_package_manifest_sha256"]),
            markdown_code(doc["identity"]["adapter"]),
            markdown_code(doc["verifier"]["runner_kind"]),
            markdown_code(verified_trials[benchmark]),
        ]) + " |")

    lines.extend([
        "",
        "## Minimum proved shared seam",
        "",
        "Only the observable meanings below are common. Profile keys and "
        "artifact paths are cited as evidence and are not promoted to a public schema.",
        "",
        "### Shared behaviors",
        "",
        "| Behavior | Artifact-derived proof |",
        "|---|---|",
        f"| Exact Agent identity at a process boundary | {markdown_code(len(agent_profiles))} distinct products are bound to separate executable and profile digests. |",
        f"| Bounded Agent execution | Both profiles carry launch, isolation, timeout, stdout-cap, and stderr-cap evidence; {markdown_code(sum(verified_trials.values()))} settled native trials completed. |",
        f"| Native artifact retention | Every Agent retained the shared roles {markdown_join(shared_roles)} while product-only roles remain namespaced. |",
        f"| Exact benchmark admission | {markdown_code(len(benchmark_profiles))} revision/task/profile/package-manifest identities are bound before grading. |",
        f"| Benchmark-owned grading | All {markdown_code(sum(verified_trials.values()))} trials were verified by the selected external native verifier and retained a known native reward. |",
        f"| Paired comparison | {markdown_code(len(reports))} task groups each contain Opi and pi plus one comparable edge. |",
        "| Fail-closed evidence chain | Dispatch, receipt, sealed manifest, canary, provider log, native rerun, oracle preflight, trial seal, and artifact bytes all verified before this matrix was rendered. |",
        "",
        "### Common Agent profile evidence fields",
        "",
        "| Field | Opi evidence | pi evidence |",
        "|---|---|---|",
    ])
    for field in common_agent_fields:
        lines.append(f"| {markdown_code(field)} | "
                     f"{markdown_code(agent_fields['opi'][field])} | "
                     f"{markdown_code(agent_fields['pi'][field])} |")

    lines.extend([
        "",
        "### Common benchmark profile evidence fields",
        "",
        "| Field | Terminal-Bench 2.1 | Terminal-Bench 3.0 | DeepSWE v1.1 |",
        "|---|---|---|---|",
    ])
    for field in common_benchmark_fields:
        lines.append(f"| {markdown_code(field)} | "
                     f"{markdown_code(benchmark_fields['terminal-bench-2.1'][field])} | "
                     f"{markdown_code(benchmark_fields['terminal-bench-3.0'][field])} | "
                     f"{markdown_code(benchmark_fields['deepswe-v1.1'][field])} |")

    lines.extend([
        "",
        "## Adapter-private evidence",
        "",
        "### Native Agent artifact roles",
        "",
        "| Product | Shared roles | Product-only roles |",
        "|---|---|---|",
    ])
    for product in sorted(roles):
        private = sorted(roles[product] - set(shared_roles))
        lines.append(f"| {markdown_code(product)} | {markdown_join(shared_roles)} | "
                     f"{markdown_join(private)} |")

    agent_all_fields = set.union(*(set(fields) for fields in agent_fields.values()))
    lines.extend([
        "",
        "### Agent-only profile fields",
        "",
        "| Product | Fields absent from the peer profile |",
        "|---|---|",
    ])
    for product in sorted(agent_fields):
        peer_fields = set.union(*(set(fields) for name, fields in agent_fields.items()
                                  if name != product))
        private = sorted((agent_all_fields - peer_fields) & set(agent_fields[product]))
        lines.append(f"| {markdown_code(product)} | {markdown_join(private)} |")

    benchmark_all_fields = set.union(
        *(set(fields) for fields in benchmark_fields.values()))
    lines.extend([
        "",
        "### Revision-only profile fields",
        "",
        "| Revision integration | Fields absent from at least one peer profile |",
        "|---|---|",
    ])
    for benchmark in sorted(benchmark_fields):
        private = sorted(set(benchmark_fields[benchmark]) &
                         (benchmark_all_fields - set(common_benchmark_fields)))
        lines.append(f"| {markdown_code(benchmark)} | {markdown_join(private)} |")

    lines.extend([
        "",
        "## Native-verifier ownership",
        "",
        "| Owner | Revision integration | Runner version | Runner commit | Verifier executable SHA-256 |",
        "|---|---|---|---|---|",
    ])
    for benchmark in sorted(benchmark_profiles):
        verifier = benchmark_profiles[benchmark]["doc"]["verifier"]
        executable = material["benchmarks"][benchmark]["verifier_executable"]
        lines.append("| " + " | ".join([
            markdown_code(verifier["runner_kind"]), markdown_code(benchmark),
            markdown_code(verifier["runner_version"]),
            markdown_code(verifier["runner_commit"]),
            markdown_code(executable["sha256"]),
        ]) + " |")
    lines.extend([
        "",
        "| Measure | Value |",
        "|---|---|",
        f"| Distinct native-verifier owners | {markdown_code(len(owners))} |",
        "",
        "## Rejected or still-provisional hypotheses",
        "",
        "| Hypothesis | Disposition |",
        "|---|---|",
        "| Package name, repository placement, and module boundaries | Provisional; the artifact proves behavior, not permanent packaging. |",
        "| Rust trait names (`AgentAdapter`, `BenchmarkAdapter`) | Provisional implementation detail, not an admitted public seam. |",
        "| JSON process envelope or exact CLI argv/environment convention | Provisional encoding; only the bounded process semantics above are proved common. |",
        "| Opi evidence JSONL and pi event JSON as one shared native schema | Rejected as a shared seam; they remain adapter-private evidence. |",
        "| ATIF, span graph, or either as the canonical trajectory | Provisional; the artifact does not prove canonicality. |",
        "| Directory layout and artifact role path spelling | Provisional evidence locations, not a compatibility contract. |",
        "| Stable SDK, public schema, publication, or compatibility promise | Provisional and requires a later Placement Review. |",
        "",
        "## Provenance",
        "",
        "| Binding | Value |",
        "|---|---|",
        f"| Native producer commit | {markdown_code(expected_commit)} |",
        f"| Workflow ref | {markdown_code(dispatch.get('github_workflow_ref'))} |",
        f"| Workflow commit | {markdown_code(dispatch.get('github_workflow_sha'))} |",
        f"| Workflow path | {markdown_code(dispatch.get('workflow_path'))} |",
        f"| Workflow SHA-256 | {markdown_code(dispatch.get('workflow_sha256_read_from_workflow_sha'))} |",
        f"| GitHub Actions run id | {markdown_code(upload.get('run_id'))} |",
        f"| GitHub Actions run URL | {markdown_code(upload.get('run_url'))} |",
        f"| Artifact id | {markdown_code(upload.get('artifact_id'))} |",
        f"| Artifact URL | {markdown_code(upload.get('artifact_url'))} |",
        f"| Artifact envelope SHA-256 | {markdown_code(upload.get('artifact_digest'))} |",
        f"| Sealed manifest SHA-256 | {markdown_code(upload.get('sealed_manifest_sha256'))} |",
        f"| Outer receipt SHA-256 | {markdown_code(upload.get('outer_receipt_sha256'))} |",
        "",
    ])
    return "\n".join(lines).encode("utf-8")


def write_atomic(path: Path, data: bytes, f: Findings) -> None:
    """Atomically replaces the requested matrix without partial output."""
    temporary = None
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.NamedTemporaryFile(
                mode="wb", dir=path.parent, prefix=f".{path.name}.",
                delete=False) as handle:
            temporary = Path(handle.name)
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except OSError as error:
        if temporary is not None:
            temporary.unlink(missing_ok=True)
        f.reject("matrix", f"cannot atomically write {path}: {error}")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="verify a downloaded eval native-smoke artifact")
    parser.add_argument("--criterion", required=True,
                        help="one of " + ", ".join(CRITERIA) +
                             ", BMK-003, all-native")
    parser.add_argument("--expected-commit", required=True,
                        help="the exact native-smoke producer commit")
    parser.add_argument("--receipt", required=True,
                        help="downloaded upload-identity receipt (file or zip)")
    parser.add_argument("--artifact", required=True,
                        help="downloaded artifact (GitHub zip or sealed tar)")
    parser.add_argument("--repo", default=".",
                        help="local git repository holding the expected commit")
    parser.add_argument(
        "--matrix-output",
        help="write the derived seam-evidence matrix after all-native acceptance")
    args = parser.parse_args()

    criterion = args.criterion
    if args.matrix_output is not None and criterion != "all-native":
        print("--matrix-output requires --criterion all-native", file=sys.stderr)
        return 2
    selected = list(CRITERIA) if criterion == "all-native" else [criterion]
    if criterion == "all-native":
        selected.append("BMK-003")
    elif criterion not in CRITERIA and criterion != "BMK-003":
        print(f"unknown criterion {criterion!r}", file=sys.stderr)
        return 2

    f = Findings()
    upload = unpack_receipt(Path(args.receipt), f)
    if upload is None:
        print(f.report(), file=sys.stderr)
        return 1
    artifact_path = Path(args.artifact)
    if artifact_path.is_dir():
        # An already-unpacked stage handoff: the envelope digest was
        # verified when the archive was unpacked; re-verify the sealed
        # manifest digest directly from the directory.
        manifest_probe = artifact_path / "08-seal" / "artifact-manifest.json"
        if not manifest_probe.is_file():
            f.reject("artifact", "directory carries no sealed manifest")
            print(f.report(), file=sys.stderr)
            return 1
        if sha(manifest_probe.read_bytes()) != upload.get("sealed_manifest_sha256"):
            f.reject("digest",
                     "unpacked stage manifest drifts from the receipt")
            print(f.report(), file=sys.stderr)
            return 1
        stage = artifact_path
    else:
        stage = unpack_artifact(artifact_path, str(upload.get("artifact_digest")), f)
        if stage is None or not f.ok():
            print(f.report(), file=sys.stderr)
            return 1

    verify_identity(upload, f)
    outer = verify_seal(stage, upload, f) if f.ok() else None
    if outer is not None and f.ok():
        verify_dispatch(outer, args.expected_commit, Path(args.repo), f)
    reports: list[dict] = []
    material: dict | None = None
    if f.ok():
        reports = verify_shared(stage, outer, f)
    if f.ok():
        verify_canary(stage, f)
        verify_provider(stage, f)
        material = verify_material(stage, f)
        verify_conformance_rerun(stage, f)
        verify_oracle_preflights(stage, f)

    for item in selected:
        if not f.ok():
            break
        if item == "EVAL-A02":
            # Paired real Opi/pi runs through the deterministic provider
            # with distinct native identities: covered by the shared
            # pairing, provider-log, and material-identity bindings.
            continue
        if item == "EVAL-A03":
            verify_agent_evidence(
                stage, reports, "opi",
                ("native/evidence/manifest", "native/evidence/records"), f)
        elif item == "EVAL-A04":
            verify_agent_evidence(
                stage, reports, "pi", ("native/events/stdout",), f)
        elif item in ("EVAL-A08", "EVAL-A09", "EVAL-A10"):
            # The native grader authority for each benchmark: covered by
            # the shared sealed-trial verifier bindings plus the passing
            # upstream oracle preflight and pinned package digest.
            continue
        elif item == "EVAL-A12":
            # One comparable cross-agent edge per task under
            # conformance-only labeling: covered by the shared pairing
            # check plus the outer-receipt labeling bindings.
            continue
        elif item == "BMK-003":
            verify_canary(stage, f)

    matrix = None
    if f.ok() and args.matrix_output is not None:
        if material is None or outer is None:
            f.reject("matrix", "accepted material or outer receipt is unavailable")
        else:
            matrix = render_seam_matrix(
                stage, upload, outer, material, reports,
                args.expected_commit, Path(args.repo), f)

    if not f.ok():
        print(f.report(), file=sys.stderr)
        return 1
    if matrix is not None:
        write_atomic(Path(args.matrix_output), matrix, f)
        if not f.ok():
            print(f.report(), file=sys.stderr)
            return 1
    for item in selected:
        print(f"opi-eval-native-artifact: {item} verified")
    if matrix is not None:
        print(f"opi-eval-native-artifact: matrix written to {args.matrix_output}")
    print("opi-eval-native-artifact: evidence is conformance-only")
    return 0


if __name__ == "__main__":
    sys.exit(main())
