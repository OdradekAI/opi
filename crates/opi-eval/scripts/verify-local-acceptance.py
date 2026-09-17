#!/usr/bin/env python3
"""Offline verifier for the redacted local acceptance evidence root.

Validates the supplied operator evidence - the redacted manifest, the six
attributable trials across the three native benchmark families, the frozen
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
import subprocess
import sys
import tempfile
import tomllib
import urllib.parse
from pathlib import Path

ACCEPTANCE_SCHEMA = "opi-eval-local-acceptance/1"
IDENTITY_CACHE_SCHEMA = "opi-eval-upstream-identity-cache/1"
VERIFICATION_SCHEMA = "opi-eval-local-acceptance-verification/1"
FAMILIES = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
EXPECTED_TRIALS = 6
HEX40 = re.compile(r"^[0-9a-f]{40}$")
HEX40_TOKEN = re.compile(r"\b[0-9a-f]{40}\b")
NATIVE_GATE = Path("bundle/artifacts/native/verifier-input.json")
NATIVE_CAPTURES = Path("bundle/artifacts/native")
# Credential-shaped names never cross into the spawned Companion.
CREDENTIAL_MARKERS = ("API_KEY", "TOKEN", "SECRET", "CREDENTIAL", "OPI_EVAL_SCRIPTED_KEY")


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


class Acceptance:
    """One verification pass over a supplied evidence root."""

    def __init__(self, root: Path, out: Path, companion: Path) -> None:
        self.root = root
        self.out = out
        self.companion = companion
        self.checks: list[dict] = []
        self.failures: list[str] = []

    def record(self, check_id: str, ok: bool, detail: str) -> None:
        self.checks.append(
            {"id": check_id, "status": "pass" if ok else "fail", "detail": detail}
        )
        if not ok:
            self.failures.append(detail)

    # -- manifest and structure --------------------------------------------

    def load_manifest(self) -> dict:
        manifest = load_json(self.root / "manifest.json", "manifest schema")
        if manifest.get("schema") != ACCEPTANCE_SCHEMA:
            raise Rejection(
                f"manifest schema: expected {ACCEPTANCE_SCHEMA}, "
                f"got {manifest.get('schema')!r}"
            )
        return manifest

    def check_trials(self, manifest: dict) -> list[dict]:
        trials = manifest.get("trials")
        if not isinstance(trials, list) or len(trials) != EXPECTED_TRIALS:
            raise Rejection(
                f"six attributable trials: manifest declares "
                f"{len(trials) if isinstance(trials, list) else 'a non-list'} entries"
            )
        for trial in trials:
            if trial.get("agent") not in ("opi", "pi"):
                raise Rejection(
                    f"six attributable trials: {trial.get('id')!r} has agent "
                    f"{trial.get('agent')!r}"
                )
        ids = [trial.get("id") for trial in trials]
        if len(set(ids)) != EXPECTED_TRIALS:
            raise Rejection(f"six attributable trials: duplicate trial ids {sorted(ids)}")
        per_family: dict[str, list[dict]] = {}
        for trial in trials:
            per_family.setdefault(trial.get("family"), []).append(trial)
        for family, entries in per_family.items():
            agents = sorted(entry["agent"] for entry in entries)
            if agents != ["opi", "pi"]:
                raise Rejection(
                    f"six attributable trials: family {family!r} carries {agents}"
                )
        if sorted(per_family) != sorted(FAMILIES):
            raise Rejection(
                f"six attributable trials: families {sorted(per_family)} do not "
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

    def check_native_evidence(self, trials: list[dict]) -> None:
        for trial in trials:
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
        known = {entry.get("agent"): entry.get("executable_sha256") for entry in builds}
        if set(known) != {"opi", "pi"}:
            raise Rejection(f"build: manifest builds {sorted(known)} lack an agent")
        for trial in trials:
            expected = known[trial["agent"]]
            observed = trial.get("attribution", {}).get("agent_executable_sha256")
            if observed != expected:
                raise Rejection(
                    f"build: trial {trial['id']!r} attributes {observed} but the "
                    f"{trial['agent']} build is {expected}"
                )

    def check_material(self, manifest: dict) -> None:
        for entry in manifest.get("material", []):
            family = entry.get("family")
            if family not in FAMILIES:
                raise Rejection(f"material digest: unknown family {family!r}")
            preserved = self.root / "runs" / family / "material.json"
            if not preserved.is_file():
                raise Rejection(f"material digest: family {family!r} preserved no material")
            observed = sha256(preserved.read_bytes())
            if observed != entry.get("material_sha256"):
                raise Rejection(
                    f"material digest: family {family!r} declares "
                    f"{entry.get('material_sha256')} but the preserved bytes hash to "
                    f"{observed}"
                )

    def check_command_exits(self, manifest: dict) -> None:
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
            self.root / "runs" / family / "run-report.json", "command exits"
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

    def check_preserved(self, manifest: dict) -> None:
        for entry in manifest.get("preserved", []):
            relative = entry.get("path", "")
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

    def check_controls(self, manifest: dict, trials: list[dict]) -> None:
        controls = manifest.get("controls", {})
        concurrency = controls.get("execution_concurrency")
        if type(concurrency) is not int or concurrency < 1:
            raise Rejection(f"run-control: controls declare no positive concurrency")
        if not isinstance(controls.get("connection_controls"), dict):
            raise Rejection(f"run-control: controls declare no frozen connection controls")
        per_family: dict[str, list[dict]] = {}
        for trial in trials:
            per_family.setdefault(trial["family"], []).append(trial)
        for family, entries in sorted(per_family.items()):
            control = load_json(
                self.root / "runs" / family / "run-control.json", "run-control"
            )
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
                self.check_output_control(controls, experiment, trial, bundle, artifact_entries)

    def check_output_control(self, controls, experiment, trial, bundle, entries):
        """Read only sealed loopback evidence; never follow its original host paths."""
        def reject(reason):
            raise Rejection(f"output-control: {trial['id']!r} {reason}")

        declared = experiment.get("model_controls")
        claims = controls["connection_controls"]
        if not isinstance(declared, dict) or claims.get("model_controls") != declared:
            reject("manifest controls differ from the sealed experiment")
        limit = declared.get("max_output_tokens")
        if type(limit) is not int or limit <= 0:
            reject("requires a positive, proven output limit")
        if experiment.get("execution", {}).get("concurrency") != controls["execution_concurrency"]:
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
                expected_model["supports_thinking" if opi else "reasoning"] = True
                expected_model["thinking_level_map" if opi else "thinkingLevelMap"] = {reasoning: reasoning}
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
                    f"three Opi trials"
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
        for name in ("pi", "harbor", "pier"):
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
        families = sorted({trial["family"] for trial in trials})
        with tempfile.TemporaryDirectory(prefix="opi-eval-acceptance-") as scratch:
            for family in families:
                run = self.root / "runs" / family
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
        self.check_oracle(trials)
        self.check_native_evidence(trials)
        self.check_bundle_digests(trials)
        self.check_builds(manifest, trials)
        self.check_material(manifest)
        self.check_command_exits(manifest)
        self.check_preserved(manifest)
        self.check_controls(manifest, trials)
        self.check_scoped_view(manifest, trials)
        self.check_upstream_identities(manifest)
        self.record(
            "evidence-structure",
            True,
            "six attributable native trials, digests, controls, exits and "
            "preserved artifacts verified offline",
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
            "checks": self.checks,
        }


def verify(root: Path, out: Path, companion: Path) -> int:
    if not root.is_dir():
        print(f"error: evidence root does not exist: {root}", file=sys.stderr)
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
    acceptance = Acceptance(root, out, companion)
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
    arguments = parser.parse_args()
    return verify(arguments.root, arguments.out, arguments.companion)


if __name__ == "__main__":
    sys.exit(main())
