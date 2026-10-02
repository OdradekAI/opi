#!/usr/bin/env python3
"""Artifact verifier for the opi-eval Linux external-lock materialization.

Audits a manually dispatched materialization run before its evidence is
admitted as the resolved Linux x86_64 external lock:

* the run-produced /2 receipt's schema, identity, and candidate-commit
  binding, including its shared-provenance references (the built Companion,
  the diagnostic config, the shared prepare command/report/material, and
  the shared conformance command/report) as contained, relative, SHA-bound
  members of the artifact;
* the downloaded GitHub artifact zip: its digest, the embedded receipt,
  the materialized closure manifest recomputed file by file, the inspected
  image graph, the shared-CLI reports and frozen native material, and the
  uniquely bound raw oracle result/CTRF and network evidence bytes;
* the committed static external lock, the pinned workflow bytes, and every
  pinned producer byte in the working tree;
* artifact expiry against the caller's check time.

The oracle evidence must show `met: true` with a native-trial-result
reward and `agent_score: false`; legacy /1 receipts and resolved locks are
rejected as superseded history (their retained bytes stay read-only).

With `--write-lock` the verifier then writes the reviewed resolved lock as a
deterministic repository artifact; with `--verify-lock` it re-derives that
document and rejects any drift. The script never touches the network and
never executes the materialization workflow.

Exit codes: 0 accepted, 2 rejected (including missing context).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import tempfile
import zipfile
from datetime import datetime, timezone
from pathlib import Path

STATIC_LOCK_SCHEMA = "opi-eval-external-lock/static/1"
RESOLVED_LOCK_SCHEMA = "opi-eval-external-lock/resolved/2"
RECEIPT_SCHEMA = "opi-eval-materialization-receipt/2"
MATERIAL_SCHEMA = "opi-eval-native-material/7"
CONFORMANCE_REPORT_SCHEMA = "opi-eval-conformance-report/1"
LOCK_ID = "opi-eval-linux-x86_64"
PLATFORM = "linux-x86_64"
RESOLVED_BY_STAGE = "external-lock-materialization"
ARTIFACT_NAME = "opi-eval-linux-lock-materialization"
DEFAULT_STATIC_LOCK = "crates/opi-eval/external-locks/static/linux-x86_64.json"

HEX40 = re.compile(r"^[0-9a-f]{40}$")
SHA256_HEX = re.compile(r"^[0-9a-f]{64}$")
REGISTRY_DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
TIMESTAMP = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")


class Rejection(Exception):
    """One fail-closed verification violation."""


def lf_sha256(data: bytes) -> str:
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Rejection(message)


def canonical_timestamp(value: object, field: str) -> str:
    require(isinstance(value, str), f"{field} is not a timestamp string: {value!r}")
    assert isinstance(value, str)
    require(TIMESTAMP.match(value) is not None, f"malformed timestamp for {field}: {value!r}")
    try:
        datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as error:
        raise Rejection(f"malformed timestamp for {field}: {value!r} ({error})") from error
    return value


def now_utc() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def load_json(path: Path, what: str) -> dict:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise Rejection(f"cannot read {what} {path}: {error}") from error
    try:
        doc = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise Rejection(f"{what} {path} is not valid UTF-8 JSON: {error}") from error
    require(isinstance(doc, dict), f"{what} {path} is not a JSON object")
    return doc


def load_static_lock(root: Path, path: Path) -> tuple[dict, str]:
    lock = load_json(path, "static lock")
    require(lock.get("schema") == STATIC_LOCK_SCHEMA, f"unsupported static lock schema: {lock.get('schema')!r}")
    require(
        lock.get("lock_id") == LOCK_ID and lock.get("platform") == PLATFORM,
        f"static lock identity mismatch: {lock.get('lock_id')!r}/{lock.get('platform')!r}",
    )
    authority = lock.get("authority")
    require(isinstance(authority, dict), "static lock has no authority object")
    digest = lf_sha256(path.read_bytes())
    return lock, digest


def audit_local_pins(root: Path, lock: dict) -> None:
    """The working tree's workflow and producer bytes must equal the pins."""
    authority = lock["authority"]
    workflow_pin = authority.get("workflow")
    require(isinstance(workflow_pin, dict), "static lock workflow pin is malformed")
    workflow_file = root / workflow_pin["path"]
    require(workflow_file.is_file(), f"pinned workflow file is missing: {workflow_pin['path']}")
    actual = lf_sha256(workflow_file.read_bytes())
    require(
        actual == workflow_pin["sha256"],
        f"workflow bytes drifted: {workflow_pin['path']} hashes to {actual}, "
        f"static lock pins {workflow_pin['sha256']}",
    )
    for producer in authority.get("producers", []):
        producer_file = root / producer["path"]
        require(producer_file.is_file(), f"pinned producer file is missing: {producer['path']}")
        actual = lf_sha256(producer_file.read_bytes())
        require(
            actual == producer["sha256"],
            f"producer bytes drifted for {producer['path']}: {actual} != pinned {producer['sha256']}",
        )


def open_artifact(path: Path, expected_digest: str) -> tuple[zipfile.ZipFile, dict[str, bytes]]:
    require(REGISTRY_DIGEST.match(expected_digest) is not None, f"malformed artifact digest: {expected_digest!r}")
    require(path.is_file(), f"artifact is not a file: {path}")
    # Downloaded upstream artifacts hash over their exact bytes (zip payload
    # is binary; LF normalization would corrupt the digest).
    actual = f"sha256:{hashlib.sha256(path.read_bytes()).hexdigest()}"
    require(
        actual == expected_digest,
        f"artifact digest mismatch: downloaded artifact hashes to {actual}, expected {expected_digest}",
    )
    try:
        archive = zipfile.ZipFile(path)
    except zipfile.BadZipFile as error:
        raise Rejection(f"artifact is not the downloaded GitHub artifact zip: {error}") from error
    names = [name for name in archive.namelist() if not name.endswith("/")]
    require(bool(names), "artifact zip is empty")
    prefixes = {name.split("/", 1)[0] for name in names}
    if len(prefixes) == 1 and any("/" in name for name in names):
        strip = next(iter(prefixes)) + "/"
        members = {name[len(strip):]: archive.read(name) for name in names}
    else:
        members = {name: archive.read(name) for name in names}
    return archive, members


def member(members: dict[str, bytes], name: str, what: str) -> bytes:
    require(name in members, f"artifact member is missing: {what} ({name})")
    return members[name]


def recompute_closure(members: dict[str, bytes]) -> tuple[str, int, int]:
    records = []
    for name, body in members.items():
        if not name.startswith("closure/"):
            continue
        rel = name[len("closure/") :]
        role = (
            "apt-index"
            if rel.startswith("apt/indexes/")
            else "apt-package"
            if rel.startswith("apt/pool/")
            else "uv-asset"
            if rel.startswith("uv/")
            else "wheel"
            if rel.startswith("wheels/")
            else "adapter-policy"
        )
        records.append(
            {
                "path": rel,
                "role": role,
                "size": len(body),
                "sha256": hashlib.sha256(body).hexdigest(),
            }
        )
    records.sort(key=lambda record: record["path"])
    manifest = hashlib.sha256(
        "".join(f"{r['path']}\t{r['role']}\t{r['size']}\t{r['sha256']}\n" for r in records).encode()
    ).hexdigest()
    return manifest, len(records), sum(r["size"] for r in records)


def shared_member(members: dict[str, bytes], reference: object, what: str) -> bytes:
    """One contained, relative, SHA-bound artifact member named by the receipt."""
    require(isinstance(reference, dict), f"receipt {what} reference is not an object: {reference!r}")
    rel = reference.get("path")
    require(
        isinstance(rel, str) and rel != "" and not rel.startswith("/") and ".." not in Path(rel).parts,
        f"receipt {what} reference is not contained and relative: {rel!r}",
    )
    body = member(members, rel, f"{what} bytes")
    digest = reference.get("sha256")
    require(
        isinstance(digest, str) and SHA256_HEX.match(digest) is not None,
        f"receipt {what} digest is malformed: {digest!r}",
    )
    require(
        hashlib.sha256(body).hexdigest() == digest,
        f"{what} digest mismatch inside the artifact",
    )
    return body


def verify_shared_commands(receipt: dict) -> None:
    """The shared CLI invocations recorded by the producer must be the
    shared prepare/conformance delegations with their required bindings."""
    shared = receipt.get("shared")
    require(isinstance(shared, dict), "receipt has no shared provenance object")

    def command_of(stage: str) -> list[str]:
        entry = shared.get(stage)
        require(isinstance(entry, dict), f"receipt shared.{stage} is not an object")
        command = entry.get("command")
        require(
            isinstance(command, list) and len(command) >= 3
            and all(isinstance(part, str) for part in command),
            f"receipt shared.{stage}.command is not a recorded argv: {command!r}",
        )
        require(
            Path(command[0]).name == "opi-eval",
            f"shared {stage} was not invoked through the Companion binary: {command[0]!r}",
        )
        require(command[1] == stage, f"shared {stage} command does not invoke {stage}: {command[1]!r}")
        return command

    prepare_command = command_of("prepare")
    for flag in ("--config", "--out", "--opi-source", "--opi-commit"):
        require(flag in prepare_command, f"shared prepare command lacks {flag}: {prepare_command!r}")
    conformance_command = command_of("conformance")
    required_pairs = [
        ("--suite", "benchmark"),
        ("--adapter", "terminal-bench-2.1"),
        ("--case", "oracle-witness"),
    ]
    for flag, value in required_pairs:
        require(
            flag in conformance_command
            and conformance_command[conformance_command.index(flag) + 1] == value,
            f"shared conformance command does not bind {flag} {value}: {conformance_command!r}",
        )
    for flag in ("--root", "--fixtures", "--provider", "--native-material"):
        require(flag in conformance_command, f"shared conformance command lacks {flag}: {conformance_command!r}")

    companion = shared.get("companion")
    require(isinstance(companion, dict), "receipt shared.companion is not an object")
    require(
        companion.get("located_by") == "cargo-compiler-artifact",
        f"Companion was not located from cargo compiler-artifact output: {companion.get('located_by')!r}",
    )
    material = shared.get("prepare", {}).get("material")
    require(isinstance(material, dict), "receipt shared.prepare.material is not an object")
    require(
        material.get("schema") == MATERIAL_SCHEMA,
        f"shared material schema is not {MATERIAL_SCHEMA}: {material.get('schema')!r}",
    )
    conformance = shared.get("conformance")
    require(isinstance(conformance, dict), "receipt shared.conformance is not an object")
    require(
        conformance.get("suite") == "benchmark"
        and conformance.get("adapter") == "terminal-bench-2.1"
        and conformance.get("case") == "oracle-witness",
        f"shared conformance selection drifted: {conformance!r}",
    )
    require(conformance.get("met") is True, "shared conformance is not met")
    require(conformance.get("agent_score") is False, "shared conformance claims an Agent score")
    require(
        conformance.get("reward_origin") == "native-trial-result",
        f"shared conformance reward origin is not native: {conformance.get('reward_origin')!r}",
    )


def build_resolved_lock(
    static_lock: dict,
    static_path: str,
    static_digest: str,
    receipt: dict,
    artifact_id: int,
    artifact_url: str,
    artifact_digest: str,
    members: dict[str, bytes],
) -> dict:
    declared = static_lock["unresolved"]
    producer_pins = [
        {"path": p["path"], "sha256": p["sha256"]} for p in static_lock["authority"]["producers"]
    ]
    pinned_images = {image["id"]: image for image in static_lock["images"]}
    resolved = []
    future = []
    for slot in declared:
        if slot["owner_stage"] == RESOLVED_BY_STAGE:
            continue
        future.append({"id": slot["id"], "owner_stage": slot["owner_stage"]})
    identities = {
        "materializer-tools": lf_sha256(
            "".join(
                f"{p['path']}\t{p['sha256']}\n" for p in sorted(producer_pins, key=lambda p: p["path"])
            ).encode("utf-8")
        ),
        "closure-artifact": receipt["closure"]["manifest_sha256"],
        "pulled-image-graphs": lf_sha256(member(members, "pulled-images.json", "inspected image graph")),
        "harbor-effective-config": receipt["oracle"]["raw_result"]["sha256"],
        "docker-network-evidence": lf_sha256(
            member(members, "oracle/network-inspection.txt", "network evidence")
        ),
        "oracle-preflight": receipt["oracle"]["ctrf"]["sha256"],
    }
    for slot in declared:
        if slot["owner_stage"] != RESOLVED_BY_STAGE:
            continue
        identity = identities.get(slot["id"])
        require(
            identity is not None,
            f"no verifier identity mapping for resolving-task slot {slot['id']}",
        )
        resolved.append({"id": slot["id"], "identity": identity})
    images = [
        {
            "id": image["id"],
            "manifest": image["manifest"],
            "config": image["config"],
            "layers": image["layers"],
        }
        for image in receipt["images"]
    ]
    return {
        "schema": RESOLVED_LOCK_SCHEMA,
        "lock_id": LOCK_ID,
        "platform": PLATFORM,
        "resolved_by_stage": RESOLVED_BY_STAGE,
        "static_lock": {"path": static_path, "sha256": static_digest},
        "workflow": {
            "ref": normalize_workflow_ref(receipt["workflow"]["ref"]),
            "sha": receipt["workflow"]["sha"],
            "path": receipt["workflow"]["path"],
            "bytes_sha256": receipt["workflow"]["bytes_sha256"],
        },
        "producers": producer_pins,
        "run": {"id": receipt["run"]["id"], "attempt": receipt["run"]["attempt"]},
        "artifact": {
            "name": ARTIFACT_NAME,
            "id": artifact_id,
            "digest": artifact_digest,
            "url": artifact_url,
            "expires_at": receipt["expires_at"],
        },
        "resolved_at": receipt["resolved_at"],
        "images": images,
        "closure": {
            "manifest_sha256": receipt["closure"]["manifest_sha256"],
            "file_count": receipt["closure"]["file_count"],
        },
        "shared": receipt["shared"],
        "oracle": {
            "status": receipt["oracle"]["status"],
            "reward": receipt["oracle"]["reward"],
            "agent_score": receipt["oracle"]["agent_score"],
            "ctrf_sha256": receipt["oracle"]["ctrf"]["sha256"],
            "raw_result_sha256": receipt["oracle"]["raw_result"]["sha256"],
        },
        "resolved": resolved,
        "future": future,
        "authority": {"admission": "digest"},
    }


def serialize_lock(doc: dict) -> bytes:
    return (json.dumps(doc, sort_keys=True, indent=2) + "\n").encode("utf-8")


def normalize_workflow_ref(value: str) -> str:
    """Reduce GitHub's qualified ``<repo>/<path>@<ref>`` workflow reference
    to the bare ``refs/...`` reference it carries."""
    if "@refs/" in value:
        return value.rsplit("@", 1)[1]
    return value


def verify(args: argparse.Namespace) -> tuple[str, str]:
    static_path = Path(args.static_lock) if args.static_lock else Path(args.root) / DEFAULT_STATIC_LOCK
    static_lock, static_digest = load_static_lock(Path(args.root), static_path)
    audit_local_pins(Path(args.root), static_lock)
    static_rel = str(static_path.resolve().relative_to(Path(args.root).resolve())).replace(os.sep, "/")

    require(HEX40.match(args.expected_commit) is not None, f"expected commit is not 40 hex: {args.expected_commit!r}")
    require(args.artifact_id > 0, f"artifact id is not a real GitHub artifact id: {args.artifact_id!r}")
    require(
        args.artifact_url.startswith("https://"),
        f"artifact url is not https: {args.artifact_url!r}",
    )

    receipt = load_json(Path(args.receipt), "materialization receipt")
    if receipt.get("schema") == "opi-eval-materialization-receipt/1":
        raise Rejection(
            "legacy /1 receipt: superseded by the shared-CLI /2 contract; "
            "retained /1 bytes stay read-only history"
        )
    require(receipt.get("schema") == RECEIPT_SCHEMA, f"unsupported receipt schema: {receipt.get('schema')!r}")
    require(
        receipt.get("lock_id") == LOCK_ID,
        f"receipt lock_id mismatch: {receipt.get('lock_id')!r} is not {LOCK_ID}",
    )
    require(
        receipt.get("platform") == PLATFORM,
        f"receipt platform mismatch: {receipt.get('platform')!r} is not {PLATFORM}",
    )
    require(
        receipt.get("candidate_commit") == args.expected_commit,
        f"receipt candidate commit {receipt.get('candidate_commit')!r} is not the expected {args.expected_commit}",
    )
    require(
        HEX40.match(str(receipt.get("candidate_commit", ""))) is not None,
        f"receipt candidate commit is not 40 hex: {receipt.get('candidate_commit')!r}",
    )

    receipt_static = receipt.get("static_lock")
    require(isinstance(receipt_static, dict), "receipt has no static_lock object")
    require(
        receipt_static.get("path") == static_rel,
        f"receipt static lock path {receipt_static.get('path')!r} is not {static_rel}",
    )
    require(
        receipt_static.get("sha256") == static_digest,
        f"receipt static digest {receipt_static.get('sha256')} does not match the committed static lock {static_digest}",
    )

    workflow_pin = static_lock["authority"]["workflow"]
    receipt_workflow = receipt.get("workflow")
    require(isinstance(receipt_workflow, dict), "receipt has no workflow object")
    require(
        receipt_workflow.get("path") == workflow_pin["path"],
        f"receipt workflow path {receipt_workflow.get('path')!r} is not the pinned {workflow_pin['path']}",
    )
    require(
        receipt_workflow.get("bytes_sha256") == workflow_pin["sha256"],
        "receipt workflow bytes digest does not match the pinned workflow",
    )
    require(
        isinstance(receipt_workflow.get("sha"), str) and HEX40.match(receipt_workflow["sha"]) is not None,
        f"receipt workflow sha is not 40 hex: {receipt_workflow.get('sha')!r}",
    )
    require(
        isinstance(receipt_workflow.get("ref"), str)
        and normalize_workflow_ref(receipt_workflow["ref"]).startswith("refs/"),
        f"receipt workflow ref does not carry a refs/ reference: {receipt_workflow.get('ref')!r}",
    )
    if args.workflow_sha is not None:
        require(
            receipt_workflow["sha"] == args.workflow_sha,
            f"workflow sha cross-check failed: receipt has {receipt_workflow['sha']}, expected {args.workflow_sha}",
        )
    if args.workflow_ref is not None:
        require(
            receipt_workflow["ref"] == args.workflow_ref,
            f"workflow ref cross-check failed: receipt has {receipt_workflow['ref']}, expected {args.workflow_ref}",
        )

    pinned_producers = sorted(
        (p["path"], p["sha256"]) for p in static_lock["authority"]["producers"]
    )
    receipt_producers = receipt.get("producers")
    require(isinstance(receipt_producers, list), "receipt has no producers list")
    observed_producers = sorted((p.get("path"), p.get("sha256")) for p in receipt_producers)
    require(
        observed_producers == pinned_producers,
        f"receipt producer set does not match the pinned producers: {observed_producers} != {pinned_producers}",
    )

    receipt_run = receipt.get("run")
    require(isinstance(receipt_run, dict), "receipt has no run object")
    require(
        isinstance(receipt_run.get("id"), int) and receipt_run["id"] > 0,
        f"receipt run id is not a real workflow run: {receipt_run.get('id')!r}",
    )
    require(
        isinstance(receipt_run.get("attempt"), int) and receipt_run["attempt"] > 0,
        f"receipt run attempt is not positive: {receipt_run.get('attempt')!r}",
    )
    if args.run_id is not None:
        require(
            receipt_run["id"] == args.run_id,
            f"run id cross-check failed: receipt has {receipt_run['id']}, expected {args.run_id}",
        )

    require(
        receipt.get("artifact_name") == ARTIFACT_NAME,
        f"receipt artifact name {receipt.get('artifact_name')!r} is not {ARTIFACT_NAME}",
    )

    resolved_at = canonical_timestamp(receipt.get("resolved_at"), "receipt.resolved_at")
    expires_at = canonical_timestamp(receipt.get("expires_at"), "receipt.expires_at")
    now = canonical_timestamp(args.now, "now")
    require(resolved_at < expires_at, f"timestamp invariant violated: resolved_at {resolved_at} is not before expires_at {expires_at}")
    require(now < expires_at, f"resolved lock expired at {expires_at} (checked at {now})")
    if args.artifact_expiry is not None:
        checked_expiry = canonical_timestamp(args.artifact_expiry, "--artifact-expiry")
        require(
            checked_expiry == expires_at,
            f"artifact expiry cross-check failed: receipt has {expires_at}, expected {checked_expiry}",
        )

    archive, members = open_artifact(Path(args.artifact), args.artifact_digest)
    with archive:
        zip_receipt = member(members, "receipt.json", "run-produced receipt")
        standalone_receipt = Path(args.receipt).read_bytes()
        require(
            lf_sha256(zip_receipt) == lf_sha256(standalone_receipt),
            "the receipt inside the artifact zip diverges from the standalone receipt",
        )

        manifest, count, total = recompute_closure(members)
        receipt_closure = receipt.get("closure")
        require(isinstance(receipt_closure, dict), "receipt has no closure object")
        require(count > 0, "artifact carries no materialized closure")
        require(
            receipt_closure.get("manifest_sha256") == manifest,
            f"closure manifest mismatch: recomputed {manifest}, receipt records {receipt_closure.get('manifest_sha256')}",
        )
        require(
            receipt_closure.get("file_count") == count,
            f"closure file count mismatch: recomputed {count}, receipt records {receipt_closure.get('file_count')}",
        )
        require(
            receipt_closure.get("total_bytes") == total,
            f"closure total bytes mismatch: recomputed {total}, receipt records {receipt_closure.get('total_bytes')}",
        )

        pulled_bytes = member(members, "pulled-images.json", "pulled image graph")
        pulled = json.loads(pulled_bytes.decode("utf-8"))
        receipt_images = receipt.get("images")
        require(isinstance(receipt_images, list) and len(receipt_images) == 1, "receipt images must record exactly one pulled image")
        pinned_images = {image["id"]: image for image in static_lock["images"]}
        image = receipt_images[0]
        require(image.get("id") in pinned_images, f"pulled image id is not pinned: {image.get('id')!r}")
        pin = pinned_images[image["id"]]
        require(
            image.get("manifest") == pin["manifest"],
            f"pulled image manifest {image.get('manifest')} is not the pinned {pin['manifest']}",
        )
        require(
            isinstance(image.get("config"), str) and REGISTRY_DIGEST.match(image["config"]) is not None,
            f"pulled image config is not a registry digest: {image.get('config')!r}",
        )
        layers = image.get("layers")
        require(isinstance(layers, list) and bool(layers), "pulled image layers are empty")
        for layer in layers:
            require(
                isinstance(layer, str) and REGISTRY_DIGEST.match(layer) is not None,
                f"pulled image layer is not a registry digest: {layer!r}",
            )
        if pin.get("config") is not None:
            require(image["config"] == pin["config"], "pulled image config does not match the pinned config")
        if pin.get("layers") is not None:
            require(image["layers"] == pin["layers"], "pulled image layers do not match the pinned layers")
        require(
            pulled == image,
            "the pulled image graph inside the artifact diverges from the receipt",
        )

        # Shared provenance: every reference must be a contained member of
        # this artifact with exactly the recorded bytes.
        verify_shared_commands(receipt)
        shared = receipt["shared"]
        shared_member(members, shared["companion"], "shared companion")
        shared_member(members, shared["config"], "diagnostic config")
        shared_member(members, shared["prepare"]["report"], "shared prepare report")
        material_bytes = shared_member(members, shared["prepare"]["material"], "shared material")
        try:
            material = json.loads(material_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise Rejection(f"shared material is not valid JSON: {error}") from error
        require(
            material.get("schema") == MATERIAL_SCHEMA,
            f"shared material schema inside the artifact is not {MATERIAL_SCHEMA}: {material.get('schema')!r}",
        )
        require(
            material.get("static_lock", {}).get("sha256") == static_digest,
            "shared material does not bind the committed static lock",
        )
        conformance_bytes = shared_member(members, shared["conformance"]["report"], "shared conformance report")
        try:
            conformance_report = json.loads(conformance_bytes.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise Rejection(f"shared conformance report is not valid JSON: {error}") from error
        require(
            conformance_report.get("schema") == CONFORMANCE_REPORT_SCHEMA,
            f"shared conformance report schema is malformed: {conformance_report.get('schema')!r}",
        )
        require(conformance_report.get("met") is True, "shared conformance report is not met")
        require(
            conformance_report.get("identity", {}).get("admitted_lock_digest") == static_digest,
            "shared conformance report did not admit the committed static lock",
        )

        receipt_oracle = receipt.get("oracle")
        require(isinstance(receipt_oracle, dict), "receipt has no oracle object")
        require(receipt_oracle.get("status") == "passed", f"oracle status is not passed: {receipt_oracle.get('status')!r}")
        reward = receipt_oracle.get("reward")
        require(
            isinstance(reward, (int, float)) and not isinstance(reward, bool) and 0.0 < float(reward) <= 1.0,
            f"oracle reward is not a passing value in (0, 1]: {reward!r}",
        )
        require(
            receipt_oracle.get("agent_score") is False,
            "oracle witness claims an Agent score",
        )
        require(
            isinstance(receipt_oracle.get("raw_result"), dict) and isinstance(receipt_oracle.get("ctrf"), dict),
            "receipt oracle object lacks its raw result/CTRF references",
        )
        raw_result = shared_member(members, receipt_oracle["raw_result"], "oracle raw result")
        try:
            json.loads(raw_result.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise Rejection(f"oracle raw result is not valid JSON: {error}") from error
        ctrf = shared_member(members, receipt_oracle["ctrf"], "oracle CTRF report")
        try:
            json.loads(ctrf.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise Rejection(f"oracle CTRF report is not valid JSON: {error}") from error

        receipt_network = receipt.get("network")
        require(isinstance(receipt_network, dict), "receipt has no network object")
        mode = receipt_network.get("mode")
        require(
            mode in ("none", "public"),
            f"network mode is not a known task-declared-mode token: {mode!r}",
        )
        require(
            receipt_network.get("basis") == "pinned-task-policy",
            f"network basis is not pinned-task-policy: {receipt_network.get('basis')!r}",
        )
        evidence = member(members, "oracle/network-inspection.txt", "network evidence")
        require(
            f"task-declared-network-mode: {mode}".encode("utf-8") in evidence,
            f"network evidence does not record the receipt's task-declared network mode: {mode}",
        )

        doc = build_resolved_lock(
            static_lock,
            static_rel,
            static_digest,
            receipt,
            args.artifact_id,
            args.artifact_url,
            args.artifact_digest,
            members,
        )
        expected = serialize_lock(doc)

    lock_path = Path(args.write_lock) if args.write_lock else Path(args.verify_lock)
    if args.write_lock:
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        handle = tempfile.NamedTemporaryFile(
            dir=str(lock_path.parent), prefix=lock_path.name + ".", suffix=".tmp", delete=False
        )
        try:
            handle.write(expected)
            handle.close()
            os.replace(handle.name, lock_path)
        except BaseException:
            handle.close()
            os.unlink(handle.name)
            raise
        reread = lock_path.read_bytes()
        require(lf_sha256(reread) == lf_sha256(expected), "written resolved lock does not round-trip")
        mode = "wrote"
    else:
        require(lock_path.is_file(), f"resolved lock to verify is missing: {lock_path}")
        actual = lock_path.read_bytes()
        require(
            lf_sha256(actual) == lf_sha256(expected),
            f"resolved lock drifted from the receipt/artifact-derived document: {lock_path}",
        )
        mode = "verified"

    summary = (
        f"opi-eval-materialization-artifact: PASS {mode} lock={lock_path} "
        f"static={static_digest[:12]} run={receipt_run['id']}/{receipt_run['attempt']} "
        f"closure={manifest[:12]} ({count} files, {total} bytes) "
        f"oracle=passed artifact={args.artifact_digest[:19]}"
    )
    return summary, manifest


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", default=".", help="repository root (default: cwd)")
    parser.add_argument("--expected-commit", required=True, help="exact the lock-materialization contract candidate commit")
    parser.add_argument("--receipt", required=True, help="downloaded run-produced receipt.json")
    parser.add_argument("--artifact", required=True, help="downloaded artifact zip")
    parser.add_argument("--artifact-id", required=True, type=int, help="GitHub artifact id")
    parser.add_argument("--artifact-url", required=True, help="https artifact URL")
    parser.add_argument("--artifact-digest", required=True, help="sha256:<64 hex> artifact zip digest")
    parser.add_argument("--artifact-expiry", default=None, help="optional canonical-UTC expiry cross-check")
    parser.add_argument("--workflow-ref", default=None, help="optional workflow ref cross-check")
    parser.add_argument("--workflow-sha", default=None, help="optional workflow sha cross-check")
    parser.add_argument("--run-id", default=None, type=int, help="optional workflow run id cross-check")
    parser.add_argument("--static-lock", default=None, help=f"static lock path (default: <root>/{DEFAULT_STATIC_LOCK})")
    parser.add_argument("--now", default=None, help="canonical-UTC check time (default: current UTC)")
    lock_mode = parser.add_mutually_exclusive_group(required=True)
    lock_mode.add_argument("--write-lock", default=None, help="write the reviewed resolved lock here")
    lock_mode.add_argument("--verify-lock", default=None, help="re-verify an existing resolved lock")
    args = parser.parse_args()
    if args.now is None:
        args.now = now_utc()
    try:
        summary, _ = verify(args)
    except Rejection as rejection:
        print(f"FAIL: {rejection}")
        return 2
    except (OSError, zipfile.BadZipFile, UnicodeDecodeError, json.JSONDecodeError) as error:
        print(f"FAIL: {error}")
        return 2
    print(summary)
    return 0


if __name__ == "__main__":
    sys.exit(main())
