#!/usr/bin/env python3
"""Offline verifier for the redacted local acceptance evidence root.

Validates the supplied operator evidence - the redacted manifest, at least
four paired trials across Terminal-Bench 2.1 and 3.0, the frozen
controls, the source/build/material/bundle digests, the preserved command
exits and provider/tool/native artifacts, the Opi-only scoped capture view,
and the cached upstream repository identities - and invokes the built
Companion `verify` and `report` commands only against the supplied family
run roots. It never launches an Agent, never accesses credentials, and never
contacts a provider; acceptance of evidence is an offline property.

Exit codes: 0 accepted, 2 rejected (including missing context).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shlex
import stat
import subprocess
import sys
import tempfile
import tomllib
import urllib.parse
from pathlib import Path

ACCEPTANCE_SCHEMA = "opi-eval-local-acceptance/3"
EXPECTED_SCHEMA = "opi-eval-acceptance-expected-identities/1"
IDENTITY_CACHE_SCHEMA = "opi-eval-upstream-identity-cache/1"
VERIFICATION_SCHEMA = "opi-eval-local-acceptance-verification/3"
# Registered acceptance platform (Section 7): other combinations stay
# visible as non-Phase platforms, never silently qualified.
ACCEPTED_PLATFORM = {"host": "windows-x86_64", "execution": "wsl2-linux"}
COMMAND_PHASES = (
    "doctor", "build", "prepare", "validate", "plan", "run", "verify", "report",
)
RFC3339 = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"
)
ID_COMPONENT = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
FAMILIES = ("terminal-bench-2.1", "terminal-bench-3.0")
EXCLUDED_FAMILIES = ("deepswe-v1.1",)
MINIMUM_TRIALS = 4
THINKING_BUDGETS = {
    "off": 0, "minimal": 1024, "low": 2048, "medium": 8192,
    "high": 16384, "xhigh": 16384, "max": 16384,
}
FIRST_CONNECTION = {
    "provider": "zhipu",
    "api": "anthropic-messages",
    "base_url": "https://open.bigmodel.cn/api/anthropic",
    "model": "glm-5.3-flash",
}
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX64 = re.compile(r"^[0-9a-f]{64}$")
HEX40_TOKEN = re.compile(r"\b[0-9a-f]{40}\b")
NATIVE_GATE = Path("bundle/artifacts/native/verifier-input.json")
NATIVE_CAPTURES = Path("bundle/artifacts/native")
MAX_ARTIFACT_BYTES = 64 * 1024 * 1024
# Credential-shaped names never cross into the spawned Companion.
CREDENTIAL_MARKERS = ("API_KEY", "TOKEN", "SECRET", "CREDENTIAL", "OPI_EVAL_SCRIPTED_KEY")
# Canonical locked acquisition routes for the pinned runners; the lock's
# commit plus this route is the repository-qualified identity.
RUNNER_REPOSITORIES = {
    "harbor": "https://github.com/harbor-framework/harbor",
    "pier": "https://github.com/datacurve-ai/pier",
}


class Rejection(Exception):
    """One fail-closed acceptance violation."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dir_digest(root: Path) -> str:
    """Recursive byte-stable manifest digest of a directory tree."""
    entries = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            entries.append(f"d {relative}")
        else:
            entries.append(f"f {relative} {sha256(path.read_bytes())}")
    return sha256("\n".join(entries).encode())


def load_json(path: Path, fragment: str) -> dict:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        raise Rejection(f"{fragment}: unreadable {path.name}: {error}") from error
    if not isinstance(value, dict):
        raise Rejection(f"{fragment}: {path.name} is not a JSON object")
    return value


def unique_record(rows: object, record_id: str, fragment: str) -> dict:
    if not isinstance(rows, list) or not isinstance(record_id, str) or not record_id:
        raise Rejection(f"{fragment}: missing record identity")
    matches = [row for row in rows if isinstance(row, dict) and row.get("id") == record_id]
    if len(matches) != 1:
        raise Rejection(
            f"{fragment}: expected one record for {record_id!r}, found {len(matches)}"
        )
    return matches[0]


def admit_trial_id(value: object) -> str:
    """A trial id is one nonempty path component: no separator of either
    platform, no "."/".." self-reference, no NUL byte. Admission happens
    before any evidence read, so an absolute, traversal, or dotted id can
    never steer a later filesystem access."""
    if not isinstance(value, str) or not value:
        raise Rejection(f"trial id: {value!r} is not a nonempty string")
    if value in (".", "..") or "/" in value or "\\" in value or "\x00" in value:
        raise Rejection(
            f"trial id: {value!r} is not a single contained path component"
        )
    return value


class Acceptance:
    """One verification pass over a supplied evidence root."""

    def __init__(self, root: Path, out: Path, companion: Path, expected_path: Path) -> None:
        self.root = root
        self.out = out
        self.companion = companion
        self.expected_path = expected_path
        self.expected: dict = {}
        self.expected_sha256 = ""
        self.freeze_sha256 = ""
        self.checks: list[dict] = []
        self.failures: list[str] = []
        self.family_roots: dict[str, Path] = {}

    def record(self, check_id: str, ok: bool, detail: str) -> None:
        self.checks.append(
            {"id": check_id, "status": "pass" if ok else "fail", "detail": detail}
        )
        if not ok:
            self.failures.append(detail)

    # -- manifest and structure --------------------------------------------

    def load_manifest(self) -> dict:
        manifest = self.load_strict_json(self.root / "manifest.json", "manifest schema")
        if not isinstance(manifest, dict):
            raise Rejection("manifest schema: manifest is not a JSON object")
        if manifest.get("schema") != ACCEPTANCE_SCHEMA:
            raise Rejection(
                f"manifest schema: expected {ACCEPTANCE_SCHEMA}, "
                f"got {manifest.get('schema')!r}"
            )
        return manifest

    def check_trials(self, manifest: dict) -> list[dict]:
        trials = manifest.get("trials")
        if not isinstance(trials, list) or len(trials) < MINIMUM_TRIALS:
            raise Rejection(
                f"paired attributable trials: require at least {MINIMUM_TRIALS}, manifest declares "
                f"{len(trials) if isinstance(trials, list) else 'a non-list'} entries"
            )
        for trial in trials:
            if not isinstance(trial, dict):
                raise Rejection("paired attributable trials: malformed trial entry")
            admit_trial_id(trial.get("id"))
            if trial.get("agent") not in ("opi", "pi"):
                raise Rejection(
                    f"paired attributable trials: {trial.get('id')!r} has agent "
                    f"{trial.get('agent')!r}"
                )
            if not isinstance(trial.get("attempt_id"), str) or not trial["attempt_id"]:
                raise Rejection(
                    f"paired attributable trials: {trial.get('id')!r} carries no "
                    f"attempt id"
                )
        ids = [trial.get("id") for trial in trials]
        if len(set(ids)) != len(trials):
            raise Rejection(f"paired attributable trials: duplicate trial ids {sorted(ids)}")
        per_family: dict[str, list[dict]] = {}
        for trial in trials:
            if trial.get("family") not in FAMILIES:
                raise Rejection(
                    f"paired attributable trials: excluded or unknown primary family {trial.get('family')!r}"
                )
            per_family.setdefault(trial.get("family"), []).append(trial)
        for family, entries in per_family.items():
            agents = [entry["agent"] for entry in entries]
            if agents.count("opi") != agents.count("pi") or not agents.count("opi"):
                raise Rejection(
                    f"paired attributable trials: family {family!r} carries {agents}"
                )
        if sorted(per_family) != sorted(FAMILIES):
            raise Rejection(
                f"paired attributable trials: families {sorted(per_family)} do not "
                f"match {sorted(FAMILIES)}"
            )
        return trials

    def check_oracle(self, trials: list[dict]) -> None:
        for trial in trials:
            attribution = trial.get("attribution", {})
            if "oracle" in json.dumps(attribution).lower() or trial["agent"] == "oracle":
                raise Rejection(
                    f"oracle: trial {trial['id']!r} carries oracle attribution"
                )

    def resolve_run_root(self, trial: dict) -> Path:
        relative = trial.get("run_root", "")
        candidate = (self.root / relative).resolve()
        try:
            candidate.relative_to(self.root.resolve())
        except ValueError as error:
            raise Rejection(
                f"run root: trial {trial['id']!r} run_root {relative!r} escapes the "
                f"evidence root"
            ) from error
        if not candidate.is_dir():
            raise Rejection(f"run root: missing {relative!r} for trial {trial['id']!r}")
        return candidate

    def admit_family_roots(self, trials: list[dict]) -> dict[str, Path]:
        """Resolve one contained family root per family before any evidence
        read. Both members of a family pair must declare the same run root,
        and every byte, semantic, and Companion consumer reuses this one
        admitted identity instead of a hardcoded canonical path."""
        for trial in trials:
            run = self.resolve_run_root(trial)
            family = trial["family"]
            admitted = self.family_roots.get(family)
            if admitted is not None and admitted != run:
                raise Rejection(
                    f"run root: family {family!r} declares conflicting roots "
                    f"{admitted} and {run}"
                )
            self.family_roots[family] = run
        return self.family_roots

    def check_native_evidence(self, trials: list[dict]) -> None:
        for trial in trials:
            # Admission at the first read seam: an unsafe id rejects here
            # even when a caller reached this check without the manifest
            # admission pass.
            admit_trial_id(trial.get("id"))
            run = self.resolve_run_root(trial)
            gate = run / "trials" / trial["id"] / NATIVE_GATE
            if not gate.is_file():
                raise Rejection(
                    f"fixture-only: trial {trial['id']!r} preserves no native "
                    f"verifier input"
                )

    def check_bundle_digests(self, trials: list[dict]) -> None:
        for trial in trials:
            run = self.resolve_run_root(trial)
            bundle_manifest = run / "trials" / trial["id"] / "bundle" / "manifest.json"
            if not bundle_manifest.is_file():
                raise Rejection(
                    f"bundle digest: trial {trial['id']!r} has no sealed bundle manifest"
                )
            observed = sha256(bundle_manifest.read_bytes())
            if observed != trial.get("bundle_digest"):
                raise Rejection(
                    f"bundle digest: trial {trial['id']!r} declares "
                    f"{trial.get('bundle_digest')} but the sealed bytes hash to "
                    f"{observed}"
                )

    def check_builds(self, manifest: dict, trials: list[dict]) -> None:
        builds = manifest.get("builds", [])
        known = {
            entry.get("agent"): (
                entry.get("executable_sha256")
                or (entry.get("executable") or {}).get("sha256")
            )
            for entry in builds
        }
        if set(known) != {"opi", "pi"} or any(
            not isinstance(value, str) for value in known.values()
        ):
            raise Rejection(f"build: manifest builds {sorted(known)} lack an agent")
        for trial in trials:
            expected = known[trial["agent"]]
            observed = trial.get("attribution", {}).get("agent_executable_sha256")
            if observed != expected:
                raise Rejection(
                    f"build: trial {trial['id']!r} attributes {observed} but the "
                    f"{trial['agent']} build is {expected}"
                )

    def check_material(self, manifest: dict, trials: list[dict]) -> None:
        """Exactly one preserved material entry per trial family, each
        recomputed against the preserved bytes, and each family's admitted
        static lock bound through the preserved bytes and the sealed
        control/integrity upstream identity (repository-qualified identity
        derives from the admitted lock bytes, never from supplied maps)."""
        entries = manifest.get("material", [])
        if not isinstance(entries, list) or not entries:
            raise Rejection("material digest: no preserved material inventory")
        by_family: dict[str, list[dict]] = {}
        for entry in entries:
            if not isinstance(entry, dict):
                raise Rejection("material digest: malformed material entry")
            family = entry.get("family")
            if family not in FAMILIES:
                raise Rejection(f"material digest: unknown family {family!r}")
            by_family.setdefault(family, []).append(entry)
        trial_families = {trial["family"] for trial in trials}
        if set(by_family) != trial_families:
            raise Rejection(
                f"material digest: families {sorted(by_family)} do not cover the "
                f"trial families {sorted(trial_families)} exactly"
            )
        self.lock_identities: dict[str, dict] = {}
        for family, rows in by_family.items():
            if len(rows) != 1:
                raise Rejection(
                    f"material digest: family {family!r} has {len(rows)} material "
                    "entries, expected exactly one"
                )
            preserved = self.family_roots[family] / "material.json"
            if not preserved.is_file():
                raise Rejection(f"material digest: family {family!r} preserved no material")
            observed = sha256(preserved.read_bytes())
            if observed != rows[0].get("material_sha256"):
                raise Rejection(
                    f"material digest: family {family!r} declares "
                    f"{rows[0].get('material_sha256')} but the preserved bytes hash to "
                    f"{observed}"
                )
            material = load_json(preserved, f"material {family!r}")
            pin = material.get("static_lock")
            if (
                not isinstance(pin, dict)
                or not isinstance(pin.get("sha256"), str)
                or not HEX64.match(pin["sha256"])
            ):
                raise Rejection(
                    f"material provenance: family {family!r} pins no static-lock digest"
                )
            self.lock_identities[family] = self.check_admitted_lock(
                manifest, family, pin["sha256"], trials
            )

    def check_admitted_lock(
        self, manifest: dict, family: str, lock_sha256: str, trials: list[dict]
    ) -> dict:
        """Locate the preserved admitted static-lock bytes by the material's
        pin, recompute the digest, agree it with every sealed
        control/integrity upstream identity of this family, and derive the
        repository-qualified upstream identities from the lock bytes."""
        preserved = manifest.get("preserved", [])
        if not isinstance(preserved, list):
            raise Rejection("preserved artifact: inventory is malformed")
        rows = [
            entry
            for entry in preserved
            if isinstance(entry, dict) and entry.get("sha256") == lock_sha256
        ]
        if len(rows) != 1:
            raise Rejection(
                f"material provenance: family {family!r} preserves "
                f"{len(rows)} copies of the admitted static lock, expected exactly one"
            )
        relative = rows[0].get("path", "")
        candidate = (self.root / relative).resolve()
        try:
            candidate.relative_to(self.root.resolve())
        except ValueError as error:
            raise Rejection(
                f"material provenance: admitted lock {relative!r} escapes the evidence root"
            ) from error
        if not candidate.is_file():
            raise Rejection(f"material provenance: admitted lock {relative!r} is missing")
        if sha256(candidate.read_bytes()) != lock_sha256:
            raise Rejection(
                f"material provenance: admitted lock {relative!r} does not hash to "
                f"the pinned digest {lock_sha256}"
            )
        for trial in trials:
            if trial["family"] != family:
                continue
            integrity = load_json(
                self.resolve_run_root(trial)
                / "trials"
                / trial["id"]
                / "bundle/artifacts/control/integrity.json",
                f"sealed integrity {trial['id']!r}",
            )
            expected_identity = f"{family}@{lock_sha256}"
            if integrity.get("upstream_identity") != expected_identity:
                raise Rejection(
                    f"material provenance: trial {trial['id']!r} sealed upstream "
                    f"identity {integrity.get('upstream_identity')!r} does not bind the "
                    f"admitted lock {expected_identity!r}"
                )
        lock = load_json(candidate, f"admitted lock {family!r}")
        subjects = lock.get("subjects")
        tools = lock.get("tools")
        if not isinstance(subjects, list) or not isinstance(tools, list):
            raise Rejection(
                f"material provenance: admitted lock {family!r} lacks subjects or tools"
            )
        pi = unique_record(subjects, "pi", "admitted lock subjects")
        harbor = unique_record(tools, "harbor", "admitted lock tools")
        identities = {}
        for name, record, url in (
            ("pi", pi, pi.get("repository_url")),
            ("harbor", harbor, RUNNER_REPOSITORIES["harbor"]),
        ):
            commit = record.get("commit")
            if not isinstance(url, str) or not isinstance(commit, str) or not HEX40.match(commit):
                raise Rejection(
                    f"material provenance: admitted lock has no {name} identity"
                )
            identities[name] = {"repository_url": url, "commit": commit}
        return identities

    def check_command_exits(self, manifest: dict) -> None:
        if "command_exits" in manifest:
            raise Rejection(
                "command exits: the weak /2 tuple table is replaced by the "
                "inline commands inventory"
            )
        exits = manifest.get("command_exits", [])
        observed = {
            (entry.get("family"), entry.get("command")): entry.get("exit")
            for entry in exits
        }
        for family in FAMILIES:
            for command in ("run", "verify", "report"):
                value = observed.get((family, command))
                if value is None:
                    raise Rejection(
                        f"command exits: family {family!r} preserves no {command} exit"
                    )
                if command != "run" and value != 0:
                    raise Rejection(
                        f"command exits: family {family!r} {command} exited {value}"
                    )
                if command == "run":
                    self.check_run_exit(family, value)

    def check_run_exit(self, family: str, value: int) -> None:
        """A paired run may settle non-zero through its comparability
        marker while every planned trial is sealed and graded natively;
        auth/infra/tool-loop failures leave trials unsealed or unverified
        and stay rejected."""
        if value == 0:
            return
        if value != 1:
            raise Rejection(f"command exits: family {family!r} run exited {value}")
        report = load_json(
            self.family_roots[family] / "run-report.json", "command exits"
        )
        trials = report.get("trials")
        if not isinstance(trials, list) or not trials:
            raise Rejection(
                f"command exits: family {family!r} run exited 1 without sealed trials"
            )
        for trial in trials:
            if not isinstance(trial, dict) or trial.get("status") != "sealed":
                raise Rejection(
                    f"command exits: family {family!r} run exited 1 with unsealed "
                    f"trial {trial.get('id') if isinstance(trial, dict) else trial!r}"
                )
            completion = trial.get("verifier", {}).get("completion")
            if not isinstance(completion, str) or not completion.startswith("verified"):
                raise Rejection(
                    f"command exits: family {family!r} run exited 1 without native "
                    f"verification for {trial.get('id')!r}"
                )

    def check_preserved(self, manifest: dict, counts: dict[str, int] | None = None) -> None:
        """The preserved inventory is mandatory, path-unique, and covers the
        required material, admitted-lock and evidence references for every
        family; every declared digest is recomputed. Contained /3 references
        must occur exactly once in the inventory."""
        entries = manifest.get("preserved", [])
        if not isinstance(entries, list) or not entries:
            raise Rejection("preserved artifact: no preserved inventory")
        seen: set[str] = set()
        required: set[str] = set()
        for family, run in self.family_roots.items():
            required.add(
                (run / "material.json").relative_to(self.root.resolve()).as_posix()
            )
        for entry in entries:
            if not isinstance(entry, dict):
                raise Rejection("preserved artifact: malformed entry")
            relative = entry.get("path", "")
            if not isinstance(relative, str) or not relative:
                raise Rejection("preserved artifact: entry declares no path")
            if relative in seen:
                raise Rejection(
                    f"preserved artifact: duplicate path {relative!r}"
                )
            seen.add(relative)
            candidate = (self.root / relative).resolve()
            try:
                candidate.relative_to(self.root.resolve())
            except ValueError as error:
                raise Rejection(
                    f"preserved artifact: {relative!r} escapes the evidence root"
                ) from error
            if not candidate.is_file():
                raise Rejection(f"preserved artifact: missing {relative!r}")
            observed = sha256(candidate.read_bytes())
            if observed != entry.get("sha256"):
                raise Rejection(
                    f"preserved artifact: {relative!r} declares {entry.get('sha256')} "
                    f"but the bytes hash to {observed}"
                )
        missing = sorted(required - seen)
        if missing:
            raise Rejection(
                f"preserved artifact: required references missing {missing}"
            )
        if counts is not None:
            for path in sorted(counts):
                if path not in seen:
                    raise Rejection(
                        f"preserved artifact: {path!r} is referenced but not "
                        f"inventoried"
                    )

    @staticmethod
    def control_block(controls: dict, family: str) -> dict:
        """Per-family control block when declared; the shared block otherwise.

        Benchmarks whose primary sources prescribe different sampling or
        output protocols freeze one block per family; the legacy shared
        block stays valid when no per-family declaration exists.
        """
        families = controls.get("families")
        if isinstance(families, dict):
            block = families.get(family)
            if not isinstance(block, dict):
                raise Rejection(
                    f"run-control: controls declare no block for family {family!r}"
                )
            return block
        return {
            "connection_controls": controls.get("connection_controls"),
            "execution_concurrency": controls.get("execution_concurrency"),
        }

    def check_controls(self, manifest: dict, trials: list[dict]) -> None:
        controls = manifest.get("controls", {})
        declared_families = controls.get("families")
        trial_families = {trial["family"] for trial in trials}
        if isinstance(declared_families, dict):
            if set(declared_families) != trial_families:
                raise Rejection(
                    "run-control: per-family controls must cover exactly the "
                    "accepted families"
                )
            blocks = declared_families
        else:
            concurrency = controls.get("execution_concurrency")
            if type(concurrency) is not int or concurrency < 1:
                raise Rejection(f"run-control: controls declare no positive concurrency")
            if not isinstance(controls.get("connection_controls"), dict):
                raise Rejection(f"run-control: controls declare no frozen connection controls")
            blocks = {family: controls for family in trial_families}
        for family, block in sorted(blocks.items()):
            concurrency = block.get("execution_concurrency")
            if type(concurrency) is not int or concurrency < 1:
                raise Rejection(
                    f"run-control: family {family!r} declares no positive concurrency"
                )
            if not isinstance(block.get("connection_controls"), dict):
                raise Rejection(
                    f"run-control: family {family!r} declares no frozen connection controls"
                )
        per_family: dict[str, list[dict]] = {}
        for trial in trials:
            per_family.setdefault(trial["family"], []).append(trial)
        for family, entries in sorted(per_family.items()):
            control = load_json(
                self.family_roots[family] / "run-control.json", "run-control"
            )
            accepted_ids = {trial["id"] for trial in entries}
            self.check_frozen_coverage(control.get("planned_trials"), accepted_ids, "planned")
            pairs: dict[tuple[str, str], list[tuple[str, str]]] = {}
            for trial in entries:
                planned = unique_record(
                    control.get("planned_trials"), trial["id"], "run-control planned trial"
                )
                bundle = self.resolve_run_root(trial) / "trials" / trial["id"] / "bundle"
                bundle_manifest = load_json(bundle / "manifest.json", "bundle manifest")
                artifact_entries = bundle_manifest.get("entries", {})
                entry = (
                    artifact_entries.get("control/experiment.json")
                    if isinstance(artifact_entries, dict)
                    else None
                )
                try:
                    experiment_bytes = (
                        bundle / "artifacts/control/experiment.json"
                    ).read_bytes()
                except OSError as error:
                    raise Rejection(
                        f"run-control: {trial['id']!r} has no readable sealed experiment"
                    ) from error
                if not isinstance(entry, dict) or sha256(experiment_bytes) != entry.get(
                    "digest"
                ):
                    raise Rejection(
                        f"experiment digest: {trial['id']!r} differs from its bundle entry"
                    )
                try:
                    experiment = json.loads(experiment_bytes)
                except (ValueError, UnicodeError) as error:
                    raise Rejection(
                        f"run-control: {trial['id']!r} has an invalid sealed experiment"
                    ) from error
                if not isinstance(experiment, dict):
                    raise Rejection(
                        f"run-control: {trial['id']!r} sealed experiment is not an object"
                    )
                self.check_frozen_coverage(experiment.get("trials"), accepted_ids, "sealed")
                selected = unique_record(
                    experiment.get("connections"), experiment.get("connection_id"),
                    "first connection",
                )
                if (
                    experiment.get("execution", {}).get("mode") != "live"
                    or {key: selected.get(key) for key in FIRST_CONNECTION} != FIRST_CONNECTION
                ):
                    raise Rejection(
                        f"first connection: {trial['id']!r} requires live Zhipu "
                        "anthropic-messages at https://open.bigmodel.cn/api/anthropic "
                        "with glm-5.3-flash"
                    )
                for recorded, sealed in (
                    ("experiment", "experiment_id"),
                    ("manifest_digest", "manifest_digest"),
                ):
                    identity = control.get(recorded)
                    if (
                        not isinstance(identity, str)
                        or not identity
                        or identity != experiment.get(sealed)
                    ):
                        raise Rejection(
                            f"run-control: {trial['id']!r} {recorded} differs "
                            f"from its sealed experiment"
                        )
                declared = unique_record(
                    experiment.get("trials"), trial["id"], "run-control sealed trial"
                )
                for field in ("subject", "task", "group"):
                    value = planned.get(field)
                    if (
                        not isinstance(value, str)
                        or not value
                        or value != declared.get(field)
                    ):
                        raise Rejection(
                            f"run-control: {trial['id']!r} planned {field} differs "
                            f"from its sealed trial"
                        )
                subject = unique_record(
                    experiment.get("subjects"), declared["subject"], "run-control sealed subject"
                )
                if subject.get("product") != trial["agent"]:
                    raise Rejection(
                        f"run-control: {trial['id']!r} subject {declared['subject']!r} "
                        f"binds product {subject.get('product')!r}, not {trial['agent']!r}"
                    )
                pairs.setdefault((declared["task"], declared["group"]), []).append(
                    (declared["subject"], trial["agent"])
                )
                self.check_output_control(controls, experiment, trial, bundle, artifact_entries)
                self.check_agent_observations(trial, bundle, artifact_entries)
            for key, members in pairs.items():
                if (
                    sorted(product for _, product in members) != ["opi", "pi"]
                    or len({subject for subject, _ in members}) != 2
                ):
                    raise Rejection(
                        f"run-control: family {family!r} task/group {key!r} is not exactly paired: {members!r}"
                    )

    @staticmethod
    def check_frozen_coverage(rows: object, accepted_ids: set[str], kind: str) -> None:
        """No frozen task/repetition slot may disappear from the evidence view.

        DeclaredTrial stores repetition in its group identity. Compare the
        entire selected ID set, then validate its task/group pairs below.
        """
        if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
            raise Rejection(f"run-control: malformed {kind} trial coverage")
        ids = [admit_trial_id(row.get("id")) for row in rows]
        if len(set(ids)) != len(ids) or set(ids) != accepted_ids:
            raise Rejection(
                f"run-control: {kind} trial coverage differs from the complete accepted trial set"
            )

    def check_agent_observations(self, trial, bundle, entries):
        """Bind live client observations to the sealed first-connection claim."""
        def reject(reason):
            raise Rejection(f"agent evidence: {trial['id']!r} {reason}")

        def sealed(key):
            path = bundle / "artifacts" / key
            for parent in (bundle, bundle / "artifacts", path.parent):
                if parent.is_symlink() or not stat.S_ISDIR(parent.lstat().st_mode):
                    reject(f"{key} has a linked or invalid artifact directory")
            metadata = path.lstat()
            if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > MAX_ARTIFACT_BYTES:
                reject(f"{key} is not a bounded regular artifact")
            with path.open("rb") as stream:
                raw = stream.read(MAX_ARTIFACT_BYTES + 1)
            if len(raw) > MAX_ARTIFACT_BYTES:
                reject(f"{key} exceeds the artifact bound")
            if sha256(raw) != entries.get(key, {}).get("digest"):
                reject(f"{key} differs from its seal")
            return raw

        try:
            terminal = json.loads(sealed("native/agent-terminal.json"))
            material = load_json(
                self.resolve_run_root(trial) / "material.json", "agent evidence material"
            )
            agent = material["agents"][trial["agent"]]
            executable = agent["executable"]["sha256"]
            if not isinstance(executable, str) or HEX64.fullmatch(executable) is None:
                reject("runtime executable identity is malformed")
            package = agent.get("runtime_package")
            package_digest, entrypoint = None, None
            if package is not None:
                package_digest, entrypoint = package["manifest_sha256"], package["entrypoint"]
                if (
                    not isinstance(package_digest, str)
                    or HEX64.fullmatch(package_digest) is None
                    or not isinstance(entrypoint, str)
                    or "\\" in entrypoint or "\x00" in entrypoint
                    or any(part in ("", ".", "..") for part in entrypoint.split("/"))
                ):
                    reject("runtime package identity is malformed")
            executable_path = f"/opi-eval-agent/{trial['agent']}/agent"
            if entrypoint is not None:
                executable_path = f"/opi-eval-agent/{trial['agent']}/runtime/{entrypoint}"
            command = shlex.quote(executable_path) + " --version"
            proof = terminal["runtime_preflight"]
            preflight_stdout = sealed("native/runtime-preflight-stdout.log")
            preflight_stderr = sealed("native/runtime-preflight-stderr.log")
            version = preflight_stdout.decode("utf-8") + preflight_stderr.decode("utf-8")
            if (
                terminal["schema"] not in (
                    "opi-eval-native-agent-terminal/2",
                    "opi-eval-native-agent-terminal/3",
                )
                or terminal["trial"] != trial["id"]
                or terminal["agent_executable_sha256"] != executable
                or executable != trial["attribution"]["agent_executable_sha256"]
                or terminal["task_dispatched"] is not True
                or not isinstance(terminal["delivered"], list)
                or proof["kind"] != "version" or proof["status"] != "passed"
                or type(proof["exit_code"]) is not int or proof["exit_code"] != 0
                or proof["reason"] is not None
                or proof["agent_executable_sha256"] != executable
                or proof["runtime_package_manifest_sha256"] != package_digest
                or proof["runtime_package_entrypoint"] != entrypoint
                or proof["command_sha256"] != sha256(command.encode())
                or proof["stdout_sha256"] != sha256(preflight_stdout)
                or proof["stderr_sha256"] != sha256(preflight_stderr)
                or proof["stdout_complete"] is not True or proof["stderr_complete"] is not True
                or proof["stdout_truncated"] is not False or proof["stderr_truncated"] is not False
                or not version.strip() or "\x00" in version
            ):
                reject("runtime preflight is not passed and bound to this trial")
            stdout = sealed("native/agent-stdout.log")
            stderr = sealed("native/agent-stderr.log")
            result = json.loads(sealed("native/trial-result.json"))
            # Follow the sealed settlement's graded classification
            # (P19-AUD-007): a benchmark-valid Agent non-zero exit or
            # task timeout - including a timeout tail without a final
            # AgentEnd after an already completed provider/tool loop -
            # stays an eligible graded execution trial with its native
            # task exception. Provider, auth, infrastructure, preflight,
            # undispatched, and invalidly graded terminals never count.
            settlement = json.loads(sealed("control/settlement.json"))
            graded = (
                settlement.get("schema") == "opi-eval-trial-settlement/1"
                and settlement.get("graded") is True
                and settlement.get("trial") == trial["id"]
            )
            valid_terminal = type(terminal["exit_code"]) is int and (
                (
                    not graded
                    and terminal["exit_code"] == 0
                    and terminal["timed_out"] is False
                    and terminal["failure_kind"] is None
                )
                or (
                    graded
                    and terminal["failure_kind"] is None
                    and terminal["timed_out"] is False
                    and terminal["exit_code"] == 0
                )
                or (
                    graded
                    and terminal["failure_kind"] == "agent-non-zero-exit"
                    and terminal["timed_out"] is False
                    and terminal["exit_code"] != 0
                )
                or (
                    graded
                    and terminal["failure_kind"] == "agent-timeout"
                    and terminal["timed_out"] is True
                )
                or (
                    # The Companion grades a clean agent-side non-success
                    # terminal (for example a pi `max_tokens` stop) as a
                    # native zero, mirroring its settlement vocabulary;
                    # provider/auth/infrastructure kinds still reject above.
                    graded
                    and terminal["failure_kind"] == "agent-outcome-not-success"
                    and terminal["timed_out"] is False
                    and terminal["exit_code"] == 0
                )
            )
            allowed_exceptions = set()
            if graded and terminal.get("failure_kind") == "agent-non-zero-exit":
                allowed_exceptions.add("NonZeroAgentExitCodeError")
            if graded and terminal.get("failure_kind") == "agent-timeout":
                allowed_exceptions.add("AgentTimeoutError")
            exception_info = result.get("exception_info")
            valid_exception = exception_info is None or (
                isinstance(exception_info, dict)
                and exception_info.get("exception_type") in allowed_exceptions
            )
            if (
                terminal["product"] != trial["agent"]
                or not valid_terminal
                or terminal["stdout_truncated"] is not False
                or terminal["stderr_truncated"] is not False
                or terminal["stdout_sha256"] != sha256(stdout)
                or terminal["stderr_sha256"] != sha256(stderr)
                or result["agent_info"]["model_info"]["name"] != "zhipu:glm-5.3-flash"
                or not valid_exception
                or (
                    terminal["schema"] == "opi-eval-native-agent-terminal/3"
                    and (
                        terminal.get("capture", {}).get("status") != "sanitized"
                        or terminal.get("capture_helper_sha256") is None
                        or terminal.get("helper_preflight", {}).get("status") != "passed"
                    )
                )
            ):
                reject("terminal, stream, or native model attribution differs")
            rows = [json.loads(line) for line in stdout.splitlines() if line.strip()]
            opi = trial["agent"] == "opi"
            events = [row["event"] for row in rows if row.get("type") == "Agent"] if opi else rows
            message_end, agent_end, tool_start, tool_end = (
                ("MessageEnd", "AgentEnd", "ToolExecutionStart", "ToolExecutionEnd") if opi
                else ("message_end", "agent_end", "tool_execution_start", "tool_execution_end")
            )
            messages = [event["message"] for event in events if event.get("type") == message_end]
            assistants = [message for message in messages if message.get("role") == "assistant"]
            # A graded timeout may end without a final Agent terminal
            # event after its provider/tool loop already completed; every
            # other shape still requires the Agent terminal event.
            has_agent_end = any(event.get("type") == agent_end for event in events)
            if not assistants or (
                not has_agent_end
                and not (graded and terminal.get("timed_out") is True)
            ):
                reject("missing provider response or Agent terminal event")
            for message in assistants:
                expected_api = "Anthropic" if opi else FIRST_CONNECTION["api"]
                if (
                    message.get("api") != expected_api
                    or any(message.get(key) != FIRST_CONNECTION[key] for key in ("provider", "model"))
                ):
                    reject("provider response differs from the first connection")
            call_key = "tool_call_id" if opi else "toolCallId"
            starts = {event.get(call_key) for event in events if event.get("type") == tool_start}
            ends = {event.get(call_key) for event in events if event.get("type") == tool_end}
            if not starts or None in starts or starts != ends:
                reject("missing or unsettled tool execution evidence")
        except (OSError, ValueError, KeyError, TypeError, AttributeError) as error:
            reject(f"missing or malformed live observations ({type(error).__name__})")

    def check_output_control(self, controls, experiment, trial, bundle, entries):
        """Read only sealed loopback evidence; never follow its original host paths."""
        def reject(reason):
            raise Rejection(f"output-control: {trial['id']!r} {reason}")

        block = self.control_block(controls, trial["family"])
        concurrency = block.get("execution_concurrency")
        claims = block.get("connection_controls")
        if type(concurrency) is not int or concurrency < 1:
            reject("declares no positive concurrency")
        if not isinstance(claims, dict):
            reject("declares no frozen connection controls")
        declared = experiment.get("model_controls")
        if not isinstance(declared, dict) or claims.get("model_controls") != declared:
            reject("manifest controls differ from the sealed experiment")
        limit = declared.get("max_output_tokens")
        if type(limit) is not int or limit <= 0:
            reject("requires a positive, proven output limit")
        if experiment.get("execution", {}).get("concurrency") != concurrency:
            reject("concurrency differs from the sealed experiment")
        selected = unique_record(
            experiment.get("connections"), experiment.get("connection_id"),
            "output-control connection",
        )
        public = {key: selected.get(key) for key in ("provider", "api", "base_url", "model")}
        if claims.get("connection") != public:
            reject("connection differs from the sealed experiment")
        key = "control/output-control.json"
        path = bundle / "artifacts" / key
        try:
            raw = path.read_bytes()
            if sha256(raw) != entries.get(key, {}).get("digest"):
                reject("wire snapshot digest differs from its seal")
            snapshot = json.loads(raw)
            evidence_text = snapshot["evidence"]
            proof = json.loads(evidence_text)
            config = snapshot["config"]
            request_text = snapshot["requests"]
            requests = [json.loads(line) for line in request_text.splitlines()]
            material = load_json(
                self.resolve_run_root(trial) / "material.json", "output-control material"
            )
            agent = material["agents"][trial["agent"]]
            if (
                proof["schema"] != "opi-eval-output-control/1"
                or proof["product"] != trial["agent"]
                or proof["exit_code"] != 0
                or type(proof["exit_code"]) is not int
                or proof["executable_sha256"] != trial["attribution"]["agent_executable_sha256"]
                or proof["executable_sha256"] != agent["executable"]["sha256"]
                or proof.get("runtime_package_manifest_sha256") != (agent.get("runtime_package") or {}).get("manifest_sha256")
                or proof.get("runtime_package_entrypoint") != (agent.get("runtime_package") or {}).get("entrypoint")
                or sha256(evidence_text.encode()) != snapshot["evidence_sha256"]
                or snapshot["evidence_sha256"] != agent["output_control_evidence"]["sha256"]
                or proof["connection"] != {key: public[key] for key in ("provider", "api", "model")}
                or proof["model_controls"] != declared
                or sha256(config.encode()) != proof["config"]["sha256"]
                or sha256(request_text.encode()) != proof["requests"]["sha256"]
            ):
                reject("client, package, projection, or wire identity differs")
            endpoint = urllib.parse.urlsplit(proof["probe_base_url"])
            if (
                endpoint.scheme != "http" or endpoint.hostname != "127.0.0.1"
                or not endpoint.port or endpoint.path or endpoint.query
                or endpoint.fragment or endpoint.username or endpoint.password
            ):
                reject("probe endpoint is not the loopback listener")
            # Compare the entire projected structure, including absence of
            # unsupported controls. Digests alone cannot detect a rewritten
            # config that still repeats the original declared controls.
            opi = trial["agent"] == "opi"
            expected_model = {"id": public["model"]}
            if opi:
                expected_model["display_name"] = f"opi-eval {public['provider']}"
            for source, target in (
                ("context_window", "context_window" if opi else "contextWindow"),
                ("max_output_tokens", "max_output_tokens" if opi else "maxTokens"),
            ):
                if type(declared[source]) is int:
                    expected_model[target] = declared[source]
            reasoning = declared["reasoning"]
            if reasoning not in ("omitted", "unknown"):
                if reasoning not in THINKING_BUDGETS:
                    reject("unknown explicit reasoning level")
                expected_model["supports_thinking" if opi else "reasoning"] = reasoning != "off"
                if opi:
                    expected_model["thinking_level_map"] = {
                        level: level == reasoning for level in THINKING_BUDGETS if level != "off"
                    }
                elif reasoning != "off":
                    expected_model["thinkingLevelMap"] = {reasoning: reasoning}
            if not opi and public["api"] != "anthropic-messages":
                sampling = {
                    name: declared[name] for name in ("temperature", "top_p")
                    if type(declared[name]) in (int, float)
                }
                if sampling:
                    expected_model["samplingParams"] = sampling
            if opi:
                expected_provider = {
                    "name": f"opi-eval {public['provider']}",
                    "base_url": proof["probe_base_url"], "api": public["api"],
                    "api_key_env": "OPI_EVAL_SCRIPTED_KEY",
                    "auth_scheme": "api-key" if public["api"] == "anthropic-messages" else "bearer",
                    "models": [expected_model],
                }
                expected_config = {
                    "providers": {"custom": {public["provider"]: expected_provider}},
                    "defaults": {"max_output_tokens": limit},
                }
                if reasoning not in ("omitted", "unknown"):
                    expected_config["thinking"] = {
                        "enabled": reasoning != "off",
                        "budget_tokens": max(THINKING_BUDGETS[reasoning], 1024),
                    }
                parsed = tomllib.loads(config)
                # Current projections omit a turn budget; historical sealed
                # projections may contain the former exact finite ceiling.
                # This checks bytes, not the executable's default loop policy.
                if "max_iterations" in parsed.get("defaults", {}):
                    expected_config["defaults"]["max_iterations"] = 4294967295
            else:
                expected_config = {"providers": {public["provider"]: {
                    "baseUrl": proof["probe_base_url"], "api": public["api"],
                    "apiKey": "dummy-scripted-credential", "models": [expected_model],
                }}}
                parsed = json.loads(config)
            if json.dumps(parsed, sort_keys=True, allow_nan=False) != json.dumps(
                expected_config, sort_keys=True, allow_nan=False
            ):
                reject("config differs from the frozen control projection")
            fields, paths = {
                "anthropic-messages": (("max_tokens",), ("/v1/messages", "/v1/messages?beta=true")),
                "openai-completions": (("max_tokens", "max_completion_tokens"), ("/chat/completions", "/v1/chat/completions")),
                "openai-responses": (("max_output_tokens",), ("/responses", "/v1/responses")),
            }[public["api"]]
            if len(requests) != 2:
                reject("missing first request or tool continuation")
            for index, request in enumerate(requests, 1):
                limits = request["output_limits"]
                if (
                    request["schema"] != "opi-eval-scripted-provider-log/1"
                    or request["seq"] != index or request["status"] != 200
                    or request["method"] != "POST" or request["path"] not in paths
                    or request["model"] != public["model"]
                    or not isinstance(limits, dict) or len(limits) != 1
                    or not any(
                        type(limits.get(field)) is int and limits[field] == limit
                        for field in fields
                    )
                    or not re.fullmatch(r"[0-9a-f]{64}", request["request_sha256"])
                ):
                    reject("observed wire output limit or request identity differs")
        except (OSError, ValueError, KeyError, TypeError, IndexError) as error:
            reject(f"missing or malformed sealed wire evidence ({type(error).__name__})")

    # -- scoped capture view -------------------------------------------------

    def check_scoped_view(self, manifest: dict, trials: list[dict]) -> None:
        view = self.root / "opi-artifact-audit"
        if not view.is_dir():
            raise Rejection("opi-artifact-audit: scoped capture view is missing")
        opi_trials = [trial for trial in trials if trial["agent"] == "opi"]
        expected_ids = {trial["id"] for trial in opi_trials}
        captures = {entry.name: entry for entry in sorted(view.iterdir())}
        capture_entries = {name: entry for name, entry in captures.items() if name in expected_ids}
        if not capture_entries:
            raise Rejection(
                "empty: the scoped capture view holds no Opi trial capture; a "
                "generic audit success is not acceptance"
            )
        for name, entry in captures.items():
            if name in expected_ids:
                continue
            if entry.is_dir():
                raise Rejection(
                    f"exhaustive: scoped view holds capture {name!r} beyond the "
                    f"accepted Opi trials"
                )
            text = entry.read_text(encoding="utf-8", errors="replace")
            for token in HEX40_TOKEN.findall(text):
                opi_commit = manifest.get("opi_repository", {}).get("commit")
                if token != opi_commit:
                    raise Rejection(
                        f"commit reference: scoped view file {name!r} references "
                        f"{token}, not the Opi commit"
                    )
        for trial in opi_trials:
            capture = captures.get(trial["id"])
            if capture is None or not capture.is_dir():
                raise Rejection(
                    f"exhaustive: no preserved capture for Opi trial {trial['id']!r}"
                )
            run = self.resolve_run_root(trial)
            original = run / "trials" / trial["id"] / NATIVE_CAPTURES
            if dir_digest(capture) != dir_digest(original):
                raise Rejection(
                    f"byte-bound: capture for {trial['id']!r} is not byte-identical "
                    f"to the original native capture"
                )

    # -- upstream identities ---------------------------------------------------

    def check_upstream_identities(self, manifest: dict) -> None:
        """Repository-qualified identities derive from the admitted lock
        bytes (authority); the operator cache is corroborating evidence
        only and can never substitute for or override the lock."""
        cache = load_json(
            self.root / "upstream-identity-cache.json", "upstream identity"
        )
        if cache.get("schema") != IDENTITY_CACHE_SCHEMA:
            raise Rejection(
                f"upstream identity: cache schema {cache.get('schema')!r} is not "
                f"{IDENTITY_CACHE_SCHEMA}"
            )
        repositories = cache.get("repositories", {})
        sources = manifest.get("sources", {})
        derived = {}
        for family in sorted(self.lock_identities):
            for name, identity in self.lock_identities[family].items():
                if name in derived and derived[name] != identity:
                    raise Rejection(
                        f"material provenance: families bind conflicting admitted "
                        f"{name} identities {derived[name]!r} and {identity!r}"
                    )
                derived[name] = identity
        for name in sorted(derived):
            record = sources.get(name)
            if not isinstance(record, dict):
                raise Rejection(f"upstream identity: no source record for {name}")
            url = record.get("repository_url")
            commit = record.get("commit")
            if not isinstance(url, str) or not isinstance(commit, str):
                raise Rejection(f"upstream identity: {name} record is malformed")
            if not HEX40.match(commit):
                raise Rejection(
                    f"upstream identity: {name} commit {commit!r} is not a commit id"
                )
            authority = derived.get(name)
            if (
                authority is None
                or url != authority["repository_url"]
                or commit != authority["commit"]
            ):
                raise Rejection(
                    f"upstream identity: {name} source record {url}@{commit} does "
                    f"not match the admitted lock identity "
                    f"{authority['repository_url'] if authority else None!r}"
                )
            known = repositories.get(url)
            if not isinstance(known, dict):
                raise Rejection(
                    f"upstream identity: repository {url!r} is not in the cached "
                    f"inventories"
                )
            if commit not in known.get("commits", {}):
                raise Rejection(
                    f"upstream identity: commit {commit} does not belong to {url!r}"
                )

    # -- /3 provenance: expected identities, freeze, commands, attempts ------

    @staticmethod
    def canonical_digest(value: object) -> str:
        """Inline-record identity: UTF-8 canonical JSON, sort_keys, no
        Unicode normalization, array order retained."""
        return sha256(
            json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
        )

    @classmethod
    def exact_fields(cls, record: object, fields: tuple[str, ...], fragment: str) -> dict:
        """Strict inline objects: unknown fields and duplicate keys reject.
        Duplicate keys are detected at the raw-text layer, so inline
        records are parsed through a rejecting loader."""
        if not isinstance(record, dict):
            raise Rejection(f"{fragment}: not a JSON object")
        missing = [field for field in fields if field not in record]
        if missing:
            raise Rejection(f"{fragment}: missing fields {missing}")
        extra = sorted(set(record) - set(fields))
        if extra:
            raise Rejection(f"{fragment}: unknown fields {extra}")
        return record

    def load_strict_json(self, path: Path, fragment: str) -> object:
        def reject_duplicates(pairs: list) -> dict:
            seen: set[str] = set()
            for key, _ in pairs:
                if key in seen:
                    raise Rejection(f"{fragment}: duplicate key {key!r}")
                seen.add(key)
            return dict(pairs)

        try:
            return json.loads(
                path.read_text(encoding="utf-8"), object_pairs_hook=reject_duplicates
            )
        except Rejection:
            raise
        except (OSError, ValueError) as error:
            raise Rejection(f"{fragment}: unreadable {path.name}: {error}") from error

    def admit_contained_path(
        self, relative: object, fragment: str, *, directory: bool = False
    ) -> Path:
        if not isinstance(relative, str) or not relative:
            raise Rejection(f"{fragment}: path is not a nonempty string")
        candidate = (self.root / relative).resolve()
        try:
            candidate.relative_to(self.root.resolve())
        except ValueError as error:
            raise Rejection(f"{fragment}: {relative!r} escapes the evidence root") from error
        metadata = candidate.lstat()
        if stat.S_ISLNK(metadata.st_mode):
            raise Rejection(f"{fragment}: {relative!r} is a link")
        if directory:
            if not stat.S_ISDIR(metadata.st_mode):
                raise Rejection(f"{fragment}: {relative!r} is not a directory")
            return candidate
        if not stat.S_ISREG(metadata.st_mode):
            raise Rejection(f"{fragment}: {relative!r} is a special file")
        if metadata.st_size > MAX_ARTIFACT_BYTES:
            raise Rejection(f"{fragment}: {relative!r} exceeds the artifact ceiling")
        return candidate

    def check_artifact_ref(self, ref: object, fragment: str, roles: set[str]) -> dict:
        """One contained {path, sha256} reference that must also occur
        exactly once in the preserved inventory."""
        record = self.exact_fields(ref, ("path", "sha256"), fragment)
        if not isinstance(record.get("sha256"), str) or not HEX64.match(record["sha256"]):
            raise Rejection(f"{fragment}: digest is not lowercase SHA-256")
        candidate = self.admit_contained_path(record.get("path"), fragment)
        observed = sha256(candidate.read_bytes())
        if observed != record["sha256"]:
            raise Rejection(
                f"{fragment}: {record.get('path')!r} declares {record['sha256']} "
                f"but the bytes hash to {observed}"
            )
        self.preserved_counts[record["path"]] = 0
        self.preserved_roles.setdefault(record["path"], set()).update(roles)
        return record

    def check_stream_ref(self, ref: object, fragment: str, roles: set[str]) -> dict:
        """Artifact reference plus an honest redaction label. Redacted
        derivatives are labelled; none claims reconstruction of unavailable
        secret-bearing originals. Empty captures are preserved and hashed."""
        record = self.exact_fields(ref, ("path", "sha256", "redaction"), fragment)
        redaction = record.get("redaction")
        if not isinstance(redaction, dict):
            raise Rejection(f"{fragment}: redaction is not an object")
        kind = redaction.get("kind")
        if kind == "none":
            extra = sorted(set(redaction) - {"kind"})
            if extra:
                raise Rejection(f"{fragment}: redaction kind none carries {extra}")
        elif kind == "redacted":
            for field in ("policy", "reason"):
                value = redaction.get(field)
                if not isinstance(value, str) or not value:
                    raise Rejection(f"{fragment}: redacted stream needs a {field}")
        else:
            raise Rejection(f"{fragment}: redaction kind {kind!r} is unknown")
        self.check_artifact_ref(
            {key: record[key] for key in ("path", "sha256")}, fragment, roles
        )
        return record

    def check_timestamp(self, value: object, fragment: str) -> str:
        if not isinstance(value, str) or not RFC3339.match(value):
            raise Rejection(f"{fragment}: timestamp {value!r} is not UTC RFC3339")
        return value

    def check_id(self, value: object, fragment: str) -> str:
        if not isinstance(value, str) or not ID_COMPONENT.match(value):
            raise Rejection(f"{fragment}: {value!r} is not one contained id component")
        return value

    def check_load_expected(self, manifest: dict) -> None:
        """Validation order step 2: load the selected independent file once,
        record its digest, compare it with the manifest declaration, and
        hash the current Companion before any spawn."""
        try:
            raw = self.expected_path.read_bytes()
        except OSError as error:
            raise Rejection(f"expected identities: unreadable: {error}") from error
        self.expected_sha256 = sha256(raw)
        try:
            self.expected = json.loads(raw.decode("utf-8"))
        except (UnicodeError, ValueError) as error:
            raise Rejection(f"expected identities: malformed: {error}") from error
        if not isinstance(self.expected, dict) or self.expected.get("schema") != EXPECTED_SCHEMA:
            raise Rejection(
                f"expected identities: schema is not {EXPECTED_SCHEMA!r}"
            )
        declared = manifest.get("expected_identities_sha256")
        if declared != self.expected_sha256:
            raise Rejection(
                f"expected identities: manifest declares {declared!r} but the "
                f"selected file hashes to {self.expected_sha256}"
            )
        if not self.companion.is_file():
            raise Rejection("expected identities: companion executable is missing")
        companion_sha = sha256(self.companion.read_bytes())
        companion_record = self.expected.get("companion")
        if not isinstance(companion_record, dict):
            raise Rejection("expected identities: no companion block")
        if companion_record.get("execution_executable_sha256") != companion_sha:
            raise Rejection(
                "expected identities: current Companion bytes differ from the "
                "independently reviewed execution identity"
            )

    def check_freeze(self, manifest: dict, trials: list[dict]) -> None:
        """Inline freeze identity and honesty. The digest binds the exact
        record; assembling a final manifest cannot retroactively create the
        original pre-run freeze."""
        freeze = manifest.get("freeze")
        if not isinstance(freeze, dict):
            raise Rejection("freeze: no inline freeze record")
        self.exact_fields(
            freeze,
            (
                "scope_id", "frozen_at_utc", "operator_label", "platform",
                "credential_references", "families", "budgets",
            ),
            "freeze",
        )
        self.check_id(freeze.get("scope_id"), "freeze scope_id")
        self.check_timestamp(freeze.get("frozen_at_utc"), "freeze frozen_at_utc")
        if not isinstance(freeze.get("operator_label"), str) or not freeze["operator_label"]:
            raise Rejection("freeze: operator label is empty")
        platform = freeze.get("platform")
        self.exact_fields(
            platform, ("host", "execution", "distribution", "engine_id", "evidence"),
            "freeze platform",
        )
        for field in ("host", "execution", "distribution", "engine_id"):
            if not isinstance(platform.get(field), str) or not platform[field]:
                raise Rejection(f"freeze platform: {field} is empty")
        if (platform["host"], platform["execution"]) != (
            ACCEPTED_PLATFORM["host"], ACCEPTED_PLATFORM["execution"]
        ):
            raise Rejection(
                f"freeze platform: {platform['host']}/{platform['execution']} is not "
                f"the registered acceptance platform; other values are not silently "
                f"qualified"
            )
        self.check_artifact_ref(platform.get("evidence"), "freeze platform evidence", {"freeze"})
        references = freeze.get("credential_references")
        if not isinstance(references, list) or not references:
            raise Rejection("freeze: no credential references")
        for row in references:
            record = self.exact_fields(row, ("connection_id", "kind", "label"), "credential")
            if record.get("kind") not in ("environment", "private-file"):
                raise Rejection(f"credential: kind {record.get('kind')!r} is unknown")
            self.check_id(record.get("connection_id"), "credential connection_id")
            if not isinstance(record.get("label"), str) or not record["label"]:
                raise Rejection("credential: label is redacted-empty")
        families = freeze.get("families")
        if not isinstance(families, list) or len(families) != len(FAMILIES):
            raise Rejection(f"freeze: families must be exactly {list(FAMILIES)}")
        declared_family_ids = set()
        for row in families:
            record = self.exact_fields(
                row,
                ("family", "run_root", "config", "plan", "material_sha256", "trial_ids", "bounds"),
                "freeze family",
            )
            family = record.get("family")
            if family not in FAMILIES:
                raise Rejection(f"freeze family: {family!r} is not admitted")
            declared_family_ids.add(family)
            run_root = self.admit_contained_path(
                record.get("run_root"), "freeze run_root", directory=True
            )
            if run_root != self.family_roots[family]:
                raise Rejection(
                    f"freeze family: {family!r} run root differs from the admitted root"
                )
            del run_root
            if record.get("plan") is not None:
                self.check_artifact_ref(record.get("plan"), f"freeze {family} plan", {"freeze"})
            self.check_artifact_ref(record.get("config"), f"freeze {family} config", {"freeze"})
            if not isinstance(record.get("material_sha256"), str) or not HEX64.match(
                record["material_sha256"]
            ):
                raise Rejection(f"freeze family: {family!r} material digest is malformed")
            if not isinstance(record.get("trial_ids"), list) or not record["trial_ids"]:
                raise Rejection(f"freeze family: {family!r} declares no trial ids")
            accepted = {trial["id"] for trial in trials if trial["family"] == family}
            if set(record["trial_ids"]) != accepted:
                raise Rejection(
                    f"freeze family: {family!r} trial ids differ from the accepted set"
                )
            if not isinstance(record.get("bounds"), dict):
                raise Rejection(f"freeze family: {family!r} bounds object is malformed")
        if declared_family_ids != set(FAMILIES):
            raise Rejection("freeze: families are not exactly the accepted pair")
        budgets = freeze.get("budgets")
        if not isinstance(budgets, dict) or set(budgets) != {"time", "usage", "spend"}:
            raise Rejection("freeze: budgets must cover time, usage and spend")
        for name, budget in budgets.items():
            record = self.exact_fields(
                budget, ("requested", "effective", "state", "evidence", "revision"),
                f"budget {name}",
            )
            for field in ("requested", "effective"):
                value = record.get(field)
                if value is None:
                    continue
                if not isinstance(value, dict) or set(value) != {"value", "unit", "hard"} and field == "requested":
                    if not isinstance(value, dict) or set(value) != {"value", "unit"}:
                        raise Rejection(f"budget {name}: {field} is malformed")
            state = record.get("state")
            if state not in ("not-requested", "enforced", "unsupported", "revised"):
                raise Rejection(f"budget {name}: state {state!r} is unknown")
            if record.get("evidence") is not None:
                self.check_artifact_ref(
                    record.get("evidence"), f"budget {name} evidence", {"freeze"}
                )
            revision = record.get("revision")
            if revision is not None:
                self.exact_fields(
                    revision,
                    ("approved_by", "approved_at_utc", "reason", "revised_requested"),
                    f"budget {name} revision",
                )
                self.check_timestamp(revision.get("approved_at_utc"), f"budget {name} revision")
                for field in ("approved_by", "reason"):
                    if not isinstance(revision.get(field), str) or not revision[field]:
                        raise Rejection(f"budget {name} revision: {field} is empty")
            if state == "not-requested" and record.get("revision") is not None:
                raise Rejection(
                    f"budget {name}: not-requested state carries a revision"
                )
        self.freeze_sha256 = self.canonical_digest(freeze)
        expected_freeze = self.expected.get("freeze_sha256")
        if expected_freeze != self.freeze_sha256:
            raise Rejection(
                f"freeze: canonical digest {self.freeze_sha256} differs from the "
                f"independently reviewed {expected_freeze!r}"
            )

    def check_builds_v3(self, manifest: dict) -> None:
        """Expanded build rows: identity/byte binding, not build causality."""
        builds = manifest.get("builds")
        if not isinstance(builds, list) or len(builds) != 2:
            raise Rejection("builds: exactly one Opi row and one pi row required")
        seen: set[str] = set()
        command_ids = set(self.commands_by_id)
        for row in builds:
            record = self.exact_fields(
                row,
                (
                    "agent", "receipt", "executable", "runtime_package",
                    "build_command_ids",
                ),
                "builds",
            )
            agent = record.get("agent")
            if agent not in ("opi", "pi") or agent in seen:
                raise Rejection(f"builds: agent {agent!r} is missing or duplicated")
            seen.add(agent)
            self.check_artifact_ref(record.get("receipt"), f"builds {agent} receipt", {"build"})
            executable = self.check_artifact_ref(
                record.get("executable"), f"builds {agent} executable", {"build"}
            )
            package = record.get("runtime_package")
            if package is not None:
                package_record = self.exact_fields(
                    package, ("path", "manifest_sha256"), f"builds {agent} package"
                )
                if not HEX64.match(package_record.get("manifest_sha256", "")):
                    raise Rejection(f"builds {agent}: package digest is malformed")
                self.admit_contained_path(
                    package_record.get("path"), f"builds {agent} package"
                )
            ids = record.get("build_command_ids")
            if not isinstance(ids, list) or not ids:
                raise Rejection(f"builds {agent}: no original build commands")
            for command_id in ids:
                if command_id not in command_ids:
                    raise Rejection(
                        f"builds {agent}: command {command_id!r} has no original record"
                    )
            expected_row = self.expected_agent(agent)
            if expected_row.get("executable_sha256") != executable["sha256"]:
                raise Rejection(
                    f"builds {agent}: executable differs from the independently "
                    f"reviewed identity"
                )
            if expected_row.get("receipt_sha256") != record["receipt"]["sha256"]:
                raise Rejection(
                    f"builds {agent}: receipt differs from the independently "
                    f"reviewed identity"
                )
        if seen != {"opi", "pi"}:
            raise Rejection("builds: one Opi row and one pi row are required")

    def expected_agent(self, agent: str) -> dict:
        rows = self.expected.get("agents")
        if not isinstance(rows, list):
            raise Rejection("expected identities: no agent rows")
        matches = [row for row in rows if isinstance(row, dict) and row.get("agent") == agent]
        if len(matches) != 1:
            raise Rejection(f"expected identities: no unique row for {agent!r}")
        return matches[0]

    def check_commands(self, manifest: dict, trials: list[dict]) -> None:
        """Original command captures: exact argv order, both streams, exit,
        timestamps, freeze binding and required phase coverage. The fresh
        verifier's own subprocess successes never replace original records."""
        commands = manifest.get("commands")
        if not isinstance(commands, list) or not commands:
            raise Rejection("commands: no original command inventory")
        self.commands_by_id: dict[str, dict] = {}
        for row in commands:
            record = self.exact_fields(
                row,
                (
                    "id", "phase", "family", "attempt_id", "argv", "cwd_label",
                    "executable_sha256", "started_at_utc", "ended_at_utc",
                    "exit_code", "stdout", "stderr", "freeze_sha256",
                    "inputs", "outputs",
                ),
                "commands",
            )
            command_id = self.check_id(record.get("id"), "command id")
            if command_id in self.commands_by_id:
                raise Rejection(f"commands: duplicate id {command_id!r}")
            if record.get("phase") not in COMMAND_PHASES:
                raise Rejection(f"commands: {command_id!r} phase is unknown")
            family = record.get("family")
            if family is not None and family not in FAMILIES + EXCLUDED_FAMILIES:
                raise Rejection(f"commands: {command_id!r} family {family!r} is unknown")
            if record.get("phase") == "run":
                if not isinstance(record.get("attempt_id"), str) or not record["attempt_id"]:
                    raise Rejection(f"commands: {command_id!r} run needs an attempt id")
            elif record.get("attempt_id") is not None:
                raise Rejection(
                    f"commands: {command_id!r} attempt id is reserved for run"
                )
            argv = record.get("argv")
            if not isinstance(argv, list) or not argv or any(
                not isinstance(part, str) for part in argv
            ):
                raise Rejection(f"commands: {command_id!r} argv is not a string list")
            if not isinstance(record.get("cwd_label"), str) or not record["cwd_label"]:
                raise Rejection(f"commands: {command_id!r} cwd label is empty")
            if not HEX64.match(record.get("executable_sha256", "")):
                raise Rejection(f"commands: {command_id!r} executable digest is malformed")
            self.check_timestamp(record.get("started_at_utc"), f"commands {command_id!r}")
            self.check_timestamp(record.get("ended_at_utc"), f"commands {command_id!r}")
            if type(record.get("exit_code")) is not int or isinstance(
                record.get("exit_code"), bool
            ):
                raise Rejection(f"commands: {command_id!r} exit is not an integer")
            self.check_stream_ref(record.get("stdout"), f"commands {command_id!r} stdout", {"command"})
            self.check_stream_ref(record.get("stderr"), f"commands {command_id!r} stderr", {"command"})
            if record.get("freeze_sha256") not in (None, self.freeze_sha256):
                raise Rejection(
                    f"commands: {command_id!r} freeze digest differs from the "
                    f"manifest freeze"
                )
            for field in ("inputs", "outputs"):
                items = record.get(field)
                if not isinstance(items, list):
                    raise Rejection(f"commands: {command_id!r} {field} is malformed")
                for item in items:
                    entry = self.exact_fields(
                        item, ("role", "artifact"), f"commands {command_id!r} {field}"
                    )
                    if not isinstance(entry.get("role"), str) or not entry["role"]:
                        raise Rejection(
                            f"commands: {command_id!r} {field} role is empty"
                        )
                    self.check_artifact_ref(
                        entry.get("artifact"),
                        f"commands {command_id!r} {field}",
                        {"command"},
                    )
            self.commands_by_id[command_id] = record
        # Required original phase coverage per accepted family.
        accepted = {trial["family"] for trial in trials}
        for family in sorted(accepted):
            phases = {
                row.get("phase")
                for row in self.commands_by_id.values()
                if row.get("family") == family
            }
            shared = {"build", "doctor"}
            for phase in ("prepare", "run", "verify", "report"):
                if phase not in phases:
                    raise Rejection(
                        f"commands: family {family!r} preserves no original {phase} "
                        f"record"
                    )
            if "plan" not in phases and "validate" not in phases:
                raise Rejection(
                    f"commands: family {family!r} preserves neither an original plan "
                    f"nor a validate record"
                )
            missing_shared = sorted(shared - phases - {""})
            del missing_shared
        # Build rows must resolve (checked again in check_builds_v3).
        # Exits: verify/report/prepare/plan/validate are zero; run keeps the
        # valid nonzero-comparability rule; failed predecessors keep theirs.
        for command_id, record in sorted(self.commands_by_id.items()):
            phase = record["phase"]
            exit_code = record["exit_code"]
            if phase in ("verify", "report", "prepare", "validate", "plan", "doctor"):
                if exit_code != 0:
                    raise Rejection(
                        f"commands: {command_id!r} {phase} exited {exit_code}"
                    )
            if phase == "run" and exit_code not in (0, 1):
                raise Rejection(
                    f"commands: {command_id!r} run exited {exit_code}"
                )
            if phase == "run":
                # A paired run may settle non-zero through its
                # comparability marker while every planned trial stays
                # sealed and natively verified; the weak /2 exit table
                # encoded the same rule, now bound to the original run
                # capture (P19-AUD-003).
                self.check_run_exit(record["family"], exit_code)

    def check_attempts(self, manifest: dict, trials: list[dict]) -> None:
        """Reciprocal accepted-trial/attempt coverage with retained
        predecessor chains and a bounded completeness attestation."""
        attempts = manifest.get("attempts")
        if not isinstance(attempts, dict):
            raise Rejection("attempts: no inline attempt record")
        self.exact_fields(
            attempts,
            ("scope_id", "declared_by", "declared_at_utc", "scope_description",
             "complete_within_scope", "records"),
            "attempts",
        )
        self.check_id(attempts.get("scope_id"), "attempts scope_id")
        for field in ("declared_by", "scope_description"):
            if not isinstance(attempts.get(field), str) or not attempts[field]:
                raise Rejection(f"attempts: {field} is empty")
        self.check_timestamp(attempts.get("declared_at_utc"), "attempts declared_at_utc")
        if type(attempts.get("complete_within_scope")) is not bool:
            raise Rejection("attempts: completeness attestation is not boolean")
        rows = attempts.get("records")
        if not isinstance(rows, list):
            raise Rejection("attempts: records is not a list")
        seen_ids: set[str] = set()
        accepted_by_attempt: dict[str, set[str]] = {}
        run_ids: set[str] = set()
        for row in rows:
            record = self.exact_fields(
                row,
                ("id", "family", "run_id", "trial_ids", "disposition",
                 "replacement_of", "command_ids", "artifacts", "missing_evidence_reason"),
                "attempt record",
            )
            attempt_id = self.check_id(record.get("id"), "attempt id")
            if attempt_id in seen_ids:
                raise Rejection(f"attempts: duplicate id {attempt_id!r}")
            seen_ids.add(attempt_id)
            family = record.get("family")
            if family not in FAMILIES + EXCLUDED_FAMILIES:
                raise Rejection(f"attempts: {attempt_id!r} family {family!r} is unknown")
            if not isinstance(record.get("run_id"), str):
                raise Rejection(f"attempts: {attempt_id!r} run id is not a string")
            for field in ("trial_ids", "command_ids", "artifacts"):
                if not isinstance(record.get(field), list):
                    raise Rejection(
                        f"attempts: {attempt_id!r} {field} is not a list"
                    )
            disposition = record.get("disposition")
            if disposition not in (
                "accepted", "failed", "cancelled", "effect-unknown", "unstarted", "scoped-out"
            ):
                raise Rejection(f"attempts: {attempt_id!r} disposition {disposition!r}")
            predecessor = record.get("replacement_of")
            if predecessor is not None:
                self.check_id(predecessor, f"attempts {attempt_id!r} predecessor")
            empty_allowed = disposition in ("unstarted",)
            for field in ("run_id", "trial_ids", "command_ids"):
                values = record.get(field) if isinstance(record.get(field), list) else [record.get(field)]
                if not empty_allowed and any(
                    value in (None, "") for value in values
                ):
                    raise Rejection(
                        f"attempts: {attempt_id!r} empty {field} outside unstarted work"
                    )
            if not isinstance(record.get("missing_evidence_reason"), str):
                raise Rejection(f"attempts: {attempt_id!r} missing-reason is not a string")
            for command_id in record.get("command_ids"):
                if command_id not in self.commands_by_id:
                    raise Rejection(
                        f"attempts: {attempt_id!r} references unknown command "
                        f"{command_id!r}"
                    )
            for artifact in record.get("artifacts"):
                self.check_artifact_ref(
                    artifact, f"attempts {attempt_id!r} artifact", {"attempt"}
                )
            if disposition == "accepted":
                accepted_by_attempt[attempt_id] = set(record.get("trial_ids"))
            for command_id in record.get("command_ids"):
                command = self.commands_by_id.get(command_id)
                if command and command.get("phase") == "run":
                    run_ids.add(command_id)
        # Every accepted trial maps reciprocally to an accepted attempt and
        # an original run record; the trial row's own attempt_id must be the
        # accepted attempt that lists it.
        accepted_trials = {trial["id"] for trial in trials}
        mapped: set[str] = set()
        for attempt_id, trial_ids in accepted_by_attempt.items():
            overlap = trial_ids & accepted_trials
            mapped |= overlap
            if overlap and overlap != trial_ids & accepted_trials:
                raise Rejection(f"attempts: {attempt_id!r} binds foreign trials")
        if mapped != accepted_trials:
            missing = sorted(accepted_trials - mapped)
            raise Rejection(
                f"attempts: accepted trials {missing} map to no accepted attempt"
            )
        for trial in trials:
            declared = trial.get("attempt_id")
            if declared not in accepted_by_attempt:
                raise Rejection(
                    f"attempts: trial {trial['id']!r} declares attempt {declared!r} "
                    f"which is not an accepted attempt"
                )
            if trial["id"] not in accepted_by_attempt[declared]:
                raise Rejection(
                    f"attempts: accepted attempt {declared!r} does not list trial "
                    f"{trial['id']!r}"
                )
        for trial in trials:
            run_commands = [
                record for record in self.commands_by_id.values()
                if record.get("phase") == "run"
                and record.get("family") == trial["family"]
            ]
            if not run_commands:
                raise Rejection(
                    f"attempts: trial {trial['id']!r} has no original run record"
                )

    # -- Companion invocation ---------------------------------------------------

    def companion_env(self) -> dict:
        env = {}
        for key, value in os.environ.items():
            if any(marker in key.upper() for marker in CREDENTIAL_MARKERS):
                continue
            env[key] = value
        return env

    def invoke_companion(self, argv: list[str]) -> None:
        result = subprocess.run(
            [str(self.companion), *argv],
            capture_output=True,
            text=True,
            timeout=600,
            env=self.companion_env(),
        )
        if result.returncode != 0:
            raise Rejection(
                f"companion {' '.join(argv[:2])} exited {result.returncode}: "
                f"{result.stderr.strip()}"
            )

    def check_companion(self, trials: list[dict]) -> None:
        if not (self.companion.is_file() and os.access(self.companion, os.X_OK)):
            raise Rejection(f"companion: {self.companion} is not an executable")
        # The Companion argv reuses the one admitted manifest-selected root
        # per family; admission is idempotent and rechecked at this seam.
        self.admit_family_roots(trials)
        families = sorted({trial["family"] for trial in trials})
        with tempfile.TemporaryDirectory(prefix="opi-eval-acceptance-") as scratch:
            for family in families:
                run = self.family_roots[family]
                self.invoke_companion(["verify", "--root", str(run)])
                self.invoke_companion(
                    [
                        "report",
                        "--root",
                        str(run),
                        "--out",
                        str(Path(scratch) / f"report-{family}.json"),
                    ]
                )

    # -- driver -----------------------------------------------------------------

    def run(self) -> dict:
        manifest = self.load_manifest()
        trials = self.check_trials(manifest)
        self.admit_family_roots(trials)
        self.check_oracle(trials)
        # /3 provenance layer (validation order steps 2-8).
        self.preserved_counts: dict[str, int] = {}
        self.preserved_roles: dict[str, set] = {}
        self.commands_by_id: dict[str, dict] = {}
        self.check_load_expected(manifest)
        self.check_freeze(manifest, trials)
        self.check_commands(manifest, trials)
        self.check_attempts(manifest, trials)
        self.check_builds_v3(manifest)
        self.check_builds(manifest, trials)
        self.check_material(manifest, trials)
        self.check_native_evidence(trials)
        self.check_bundle_digests(trials)
        self.check_preserved(manifest, self.preserved_counts)
        self.check_controls(manifest, trials)
        self.check_scoped_view(manifest, trials)
        self.check_upstream_identities(manifest)
        self.record(
            "evidence-structure",
            True,
            "paired native trials, complete frozen coverage, digests, controls, "
            "original commands, freeze, attempts and preserved artifacts verified "
            "offline",
        )
        self.check_companion(trials)
        self.record(
            "companion", True, "built Companion verify and report accepted every family root"
        )
        return {
            "schema": VERIFICATION_SCHEMA,
            "accepted": True,
            "root": str(self.root),
            "trials": [trial["id"] for trial in trials],
            "families": sorted({trial["family"] for trial in trials}),
            "excluded_families": list(EXCLUDED_FAMILIES),
            "expected_identities_sha256": self.expected_sha256,
            "freeze_sha256": self.freeze_sha256,
            "limit": (
                "operator-approved identity/byte consistency does not "
                "cryptographically establish build causality or completeness "
                "beyond the declared invocation scope"
            ),
            "checks": self.checks,
        }


def verify(root: Path, out: Path, companion: Path, expected_path: Path) -> int:
    if not root.is_dir():
        print(f"error: evidence root does not exist: {root}", file=sys.stderr)
        return 2
    if expected_path.resolve() == (root.resolve() / expected_path.name):
        print(
            "error: expected-identities file must be selected outside the "
            "evidence root",
            file=sys.stderr,
        )
        return 2
    if out.exists():
        print(f"error: output path already exists: {out}", file=sys.stderr)
        return 2
    try:
        out.resolve().relative_to(root.resolve())
        print(
            f"error: output path lies inside the evidence root: {out}", file=sys.stderr
        )
        return 2
    except ValueError:
        pass
    acceptance = Acceptance(root, out, companion, expected_path)
    try:
        report = acceptance.run()
    except Rejection as rejection:
        acceptance.record("evidence-structure", False, str(rejection))
        report = {
            "schema": VERIFICATION_SCHEMA,
            "accepted": False,
            "root": str(root),
            "trials": [],
            "families": [],
            "excluded_families": list(EXCLUDED_FAMILIES),
            "checks": acceptance.checks,
        }
        print(f"error: {rejection}", file=sys.stderr)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        summary = dict(report)
        summary.pop("checks")
        print(json.dumps(summary, sort_keys=True))
        return 2
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary = dict(report)
    summary.pop("checks")
    print(json.dumps(summary, sort_keys=True))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", required=True, type=Path, help="redacted evidence root")
    parser.add_argument("--out", required=True, type=Path, help="verification report path")
    parser.add_argument(
        "--companion", required=True, type=Path, help="built opi-eval executable"
    )
    parser.add_argument(
        "--expected-identities",
        required=True,
        type=Path,
        help="independently reviewed expected-identity file "
        "(opi-eval-acceptance-expected-identities/1)",
    )
    arguments = parser.parse_args()
    return verify(
        arguments.root,
        arguments.out,
        arguments.companion,
        arguments.expected_identities,
    )


if __name__ == "__main__":
    sys.exit(main())
