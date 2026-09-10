#!/usr/bin/env python3
"""Deterministic regressions for the offline local-acceptance verifier.

The verifier reads one supplied redacted evidence root and never launches an
Agent, accesses credentials, or contacts a provider. These tests exercise its
structural logic and its Companion invocation contract with a recording stub;
the genuine three-family native acceptance path over the built Companion is
proven by the Rust test in crates/opi-eval/tests/native_driver.rs.
"""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
VERIFIER = SCRIPTS / "verify-local-acceptance.py"

FAMILIES = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
PI_REPO = "https://github.com/badlogic/pi-mono"
HARBOR_REPO = "https://github.com/harbor/harbor"
PIER_REPO = "https://github.com/princeton-nlp/SWE-agent"
PI_COMMIT = "a" * 40
HARBOR_COMMIT = "b" * 40
PIER_COMMIT = "c" * 40
CONNECTION = {"id": "primary", "provider": "scripted", "api": "anthropic-messages", "base_url": "http://127.0.0.1:48127", "model": "eval"}
MODEL_CONTROLS = {"temperature": "omitted", "top_p": "omitted", "max_output_tokens": 16384, "context_window": 1000000, "reasoning": "omitted", "required": ["max_output_tokens"]}


def synthetic_output_control(product, experiment):
    """Structural fixture only; real clients run in the ignored Rust matrix."""
    if product == "opi":
        config = '''[defaults]
max_output_tokens = 16384
[providers.custom.scripted]
name = "opi-eval scripted"
base_url = "http://127.0.0.1:48127"
api = "anthropic-messages"
api_key_env = "OPI_EVAL_SCRIPTED_KEY"
auth_scheme = "api-key"
[[providers.custom.scripted.models]]
id = "eval"
display_name = "opi-eval scripted"
max_output_tokens = 16384
context_window = 1000000
'''
    else:
        config = json.dumps({"providers": {"scripted": {"baseUrl": CONNECTION["base_url"], "api": CONNECTION["api"], "apiKey": "dummy-scripted-credential", "models": [{"id": "eval", "maxTokens": 16384, "contextWindow": 1000000}]}}})
    requests = "".join(json.dumps({"schema": "opi-eval-scripted-provider-log/1", "seq": seq, "status": 200, "method": "POST", "path": "/v1/messages", "model": "eval", "output_limits": {"max_tokens": 16384}, "request_sha256": "d" * 64}) + "\n" for seq in (1, 2))
    proof = {"schema": "opi-eval-output-control/1", "product": product, "executable_sha256": "e" * 64, "runtime_package_manifest_sha256": None, "connection": {key: CONNECTION[key] for key in ("provider", "api", "model")}, "model_controls": experiment["model_controls"], "probe_base_url": CONNECTION["base_url"], "exit_code": 0, "config": {"path": "synthetic-config", "sha256": sha256(config.encode())}, "requests": {"path": "synthetic-requests", "sha256": sha256(requests.encode())}}
    evidence = json.dumps(proof)
    return {"evidence": evidence, "evidence_sha256": sha256(evidence.encode()), "config": config, "requests": requests}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dir_digest(root: Path) -> str:
    """Recursive LF-stable manifest digest of a directory tree."""
    entries = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            entries.append(f"d {relative}")
        else:
            entries.append(f"f {relative} {sha256(path.read_bytes())}")
    return sha256("\n".join(entries).encode())


def write_exec(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")
    mode = path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    os.chmod(path, mode)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n"
    )


def companion_stub(record: Path) -> str:
    return (
        "#!/bin/sh\n"
        'set -eu\n'
        f'record="{record}"\n'
        'case "$1 $2" in\n'
        '  "verify --root") [ "$#" -eq 3 ] || { echo "verify argv drift" >&2; exit 9; } ;;\n'
        '  "report --root") [ "$#" -eq 5 ] && [ "$4" = "--out" ] || { echo "report argv drift" >&2; exit 9; } ;;\n'
        '  *) echo "unexpected companion invocation: $*" >&2; exit 9 ;;\n'
        "esac\n"
        '# no credential material may cross the verifier boundary\n'
        'for name in OPENAI_API_KEY ANTHROPIC_API_KEY OPI_EVAL_SCRIPTED_KEY; do\n'
        '  if [ -n "$(eval "echo \\$$name")" ]; then echo "credential env crossed: $name" >&2; exit 9; fi\n'
        "done\n"
        'printf \'%s\\n\' "$*" >> "$record"\n'
        "exit 0\n"
    )


class EvidenceRoot:
    """A synthetic but structurally complete redacted evidence root."""

    def __init__(
        self, base: Path, subject_ids: tuple[str, str] = ("candidate-opi", "baseline-pi")
    ) -> None:
        self.root = base / "evidence"
        self.root.mkdir(parents=True)
        self.outside = base / "out"
        self.stub_record = base / "companion-invocations.txt"
        self.companion = base / "companion-stub.sh"
        write_exec(self.companion, companion_stub(self.stub_record))
        self.trials = []
        self.runs = {}
        self.subject_ids = subject_ids
        for index, family in enumerate(FAMILIES):
            run = self.root / "runs" / family
            opi_id = f"trial-opi-{index + 1}"
            pi_id = f"trial-pi-{index + 1}"
            self._stage_family(run, family, opi_id, pi_id)
            self.runs[family] = {
                "run": run,
                "opi": opi_id,
                "pi": pi_id,
                "bundle_digests": {},
            }
        self.identity_cache = {
            "schema": "opi-eval-upstream-identity-cache/1",
            "repositories": {
                PI_REPO: {"commits": {PI_COMMIT: {"tree": "1" * 40}}},
                HARBOR_REPO: {"commits": {HARBOR_COMMIT: {"tree": "2" * 40}}},
                PIER_REPO: {"commits": {PIER_COMMIT: {"tree": "3" * 40}}},
            },
        }
        write_json(self.root / "upstream-identity-cache.json", self.identity_cache)
        self._stage_scoped_view()
        self._write_manifest()

    def _stage_family(self, run: Path, family: str, opi_id: str, pi_id: str) -> None:
        declared = [
            {"id": opi_id, "subject": self.subject_ids[0], "task": family, "group": "g"},
            {"id": pi_id, "subject": self.subject_ids[1], "task": family, "group": "g"},
        ]
        experiment = {
            "schema": "opi-eval-experiment/2",
            "experiment_id": f"native-acceptance-{family}",
            "manifest_digest": sha256(family.encode()),
            "subjects": [
                {"id": self.subject_ids[0], "product": "opi", "version": "0.8.2"},
                {"id": self.subject_ids[1], "product": "pi", "version": "0.85.1"},
            ],
            "trials": declared,
            "connection_id": "primary",
            "connections": [CONNECTION],
            "model_controls": MODEL_CONTROLS,
            "execution": {"concurrency": 2},
        }
        write_json(
            run / "run-control.json",
            {
                "schema": "opi-eval-run-control/1",
                "experiment": experiment["experiment_id"],
                "manifest_digest": experiment["manifest_digest"],
                "planned_trials": declared,
            },
        )
        material = {"schema": "preserved-material", "agents": {}}
        for trial, product in ((opi_id, "opi"), (pi_id, "pi")):
            bundle = run / "trials" / trial / "bundle"
            native = bundle / "artifacts" / "native"
            write_json(
                native / "verifier-input.json",
                {"environment_mode": "separate", "collected": []},
            )
            (native / "agent-stdout.log").write_text(
                f"{family} {product} native capture bytes\n", encoding="utf-8"
            )
            write_json(
                bundle / "intent.json",
                {"schema": "opi-eval-intent/1", "product": product},
            )
            write_json(
                bundle / "artifacts/control/experiment.json",
                experiment,
            )
            snapshot = synthetic_output_control(product, experiment)
            wire_path = bundle / "artifacts/control/output-control.json"
            write_json(wire_path, snapshot)
            material["agents"][product] = {"executable": {"sha256": "e" * 64}, "output_control_evidence": {"sha256": snapshot["evidence_sha256"]}}
            write_json(
                bundle / "manifest.json",
                {
                    "schema": "opi-eval-bundle-manifest/1",
                    "entries": {
                        "control/output-control.json": {"digest": sha256(wire_path.read_bytes())},
                        "control/experiment.json": {
                            "digest": sha256(
                                (bundle / "artifacts/control/experiment.json").read_bytes()
                            )
                        }
                    },
                },
            )
        write_json(run / "material.json", material)

    def rewrite_sealed_experiment(self, family: str, experiment: dict) -> None:
        """Bind a changed synthetic contract so semantic checks see intact digests."""
        run = self.runs[family]["run"]
        for trial in run.joinpath("trials").iterdir():
            bundle = trial / "bundle"
            path = bundle / "artifacts/control/experiment.json"
            write_json(path, experiment)
            manifest = json.loads((bundle / "manifest.json").read_text())
            manifest["entries"]["control/experiment.json"]["digest"] = sha256(
                path.read_bytes()
            )
            write_json(bundle / "manifest.json", manifest)
        self._write_manifest()

    def _stage_scoped_view(self) -> None:
        view = self.root / "opi-artifact-audit"
        for family, entry in self.runs.items():
            native = entry["run"] / "trials" / entry["opi"] / "bundle" / "artifacts" / "native"
            target = view / entry["opi"]
            target.mkdir(parents=True)
            for path in sorted(native.iterdir()):
                (target / path.name).write_bytes(path.read_bytes())

    def _write_manifest(self) -> None:
        trials = []
        for family, entry in self.runs.items():
            for key, agent in (("opi", "opi"), ("pi", "pi")):
                trial = entry[key]
                bundle_manifest = (
                    entry["run"] / "trials" / trial / "bundle" / "manifest.json"
                )
                trials.append(
                    {
                        "id": trial,
                        "agent": agent,
                        "family": family,
                        "run_root": f"runs/{family}",
                        "bundle_digest": sha256(bundle_manifest.read_bytes()),
                        "attribution": {"agent_executable_sha256": "e" * 64},
                    }
                )
        manifest = {
            "schema": "opi-eval-local-acceptance/1",
            "operator": {"redacted": True, "label": "local-acceptance-operator"},
            "opi_repository": {"url": "https://github.com/OdradekAI/opi", "commit": "9" * 40},
            "controls": {
                "execution_concurrency": 2,
                "connection_controls": {"connection": {key: CONNECTION[key] for key in ("provider", "api", "base_url", "model")}, "model_controls": MODEL_CONTROLS},
            },
            "trials": trials,
            "sources": {
                "pi": {"repository_url": PI_REPO, "commit": PI_COMMIT},
                "harbor": {"repository_url": HARBOR_REPO, "commit": HARBOR_COMMIT},
                "pier": {"repository_url": PIER_REPO, "commit": PIER_COMMIT},
            },
            "builds": [
                {"agent": "opi", "executable_sha256": "e" * 64},
                {"agent": "pi", "executable_sha256": "e" * 64},
            ],
            "material": [],
            "command_exits": [],
            "preserved": [],
        }
        for family, entry in self.runs.items():
            for command in ("run", "verify", "report"):
                manifest["command_exits"].append(
                    {"family": family, "command": command, "exit": 0}
                )
            manifest["preserved"].append(
                {
                    "role": "material",
                    "path": f"runs/{family}/material.json",
                    "sha256": sha256((entry["run"] / "material.json").read_bytes()),
                }
            )
            manifest["material"].append(
                {
                    "family": family,
                    "material_sha256": sha256(
                        (entry["run"] / "material.json").read_bytes()
                    ),
                }
            )
        write_json(self.root / "manifest.json", manifest)
        self.manifest_path = self.root / "manifest.json"

    def verify(self) -> subprocess.CompletedProcess:
        out = self.outside / f"report-{id(self):x}"
        return subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--root",
                str(self.root),
                "--out",
                str(out),
                "--companion",
                str(self.companion),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )

    def load_manifest(self) -> dict:
        return json.loads(self.manifest_path.read_text(encoding="utf-8"))

    def save_manifest(self, manifest: dict) -> None:
        write_json(self.manifest_path, manifest)


class AcceptanceVerifierTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.base = Path(self._tmp.name)
        self.evidence = EvidenceRoot(self.base)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def accept(self) -> dict:
        result = self.evidence.verify()
        self.assertEqual(
            result.returncode,
            0,
            f"expected acceptance, stderr: {result.stderr}\nstdout: {result.stdout}",
        )
        report = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertTrue(report["accepted"])
        return report

    def reject(self, fragment: str) -> dict:
        result = self.evidence.verify()
        self.assertEqual(
            result.returncode,
            2,
            f"expected rejection, stdout: {result.stdout}\nstderr: {result.stderr}",
        )
        self.assertIn(fragment, result.stderr)
        report = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertFalse(report["accepted"])
        return report

    # -- acceptance ---------------------------------------------------------

    def test_genuine_structured_root_is_accepted(self) -> None:
        report = self.accept()
        self.assertEqual(report["schema"], "opi-eval-local-acceptance-verification/1")
        self.assertEqual(len(report["trials"]), 6)
        out_root = self.evidence.outside
        written = [path.name for path in sorted(out_root.iterdir())]
        self.assertEqual(written, [f"report-{id(self.evidence):x}"])

    def test_companion_invoked_once_per_family_for_verify_and_report(self) -> None:
        self.accept()
        invocations = self.evidence.stub_record.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(invocations), 6)
        for family in FAMILIES:
            self.assertIn(
                f"verify --root {self.evidence.root / 'runs' / family}", invocations
            )
            reports = [
                line
                for line in invocations
                if line.startswith(f"report --root {self.evidence.root / 'runs' / family}")
            ]
            self.assertEqual(len(reports), 1)
            self.assertIn("--out", reports[0])

    def test_fresh_subject_labels_are_accepted(self) -> None:
        self.evidence = EvidenceRoot(
            self.base / "replacement",
            subject_ids=("candidate-opi-replacement", "baseline-pi-replacement"),
        )
        self.accept()

    # -- request-level rejections -------------------------------------------

    def test_missing_root_is_rejected(self) -> None:
        self.evidence.root.rename(self.base / "moved")
        result = self.evidence.verify()
        self.assertEqual(result.returncode, 2)
        self.assertIn("evidence root does not exist", result.stderr)

    def test_existing_output_is_rejected(self) -> None:
        out = self.evidence.outside / f"report-{id(self.evidence):x}"
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("prior", encoding="utf-8")
        result = self.evidence.verify()
        self.assertEqual(result.returncode, 2)
        self.assertIn("output path already exists", result.stderr)

    def test_output_inside_root_is_rejected(self) -> None:
        out = self.evidence.root / "nested-report.json"
        result = subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--root",
                str(self.evidence.root),
                "--out",
                str(out),
                "--companion",
                str(self.evidence.companion),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("lies inside the evidence root", result.stderr)

    # -- manifest rejections -------------------------------------------------

    def test_wrong_schema_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["schema"] = "something-else/1"
        self.evidence.save_manifest(manifest)
        self.reject("manifest schema")

    def test_missing_trial_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"] = manifest["trials"][:5]
        self.evidence.save_manifest(manifest)
        self.reject("six attributable trials")

    def test_duplicate_trial_id_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][2]["id"] = manifest["trials"][0]["id"]
        self.evidence.save_manifest(manifest)
        self.reject("six attributable trials")

    def test_oracle_attribution_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][0]["attribution"]["agent_executable_sha256"] = "oracle"
        self.evidence.save_manifest(manifest)
        self.reject("oracle")

    def test_fixture_only_evidence_is_rejected(self) -> None:
        family = FAMILIES[0]
        entry = self.evidence.runs[family]
        for key in ("opi", "pi"):
            native = entry["run"] / "trials" / entry[key] / "bundle" / "artifacts" / "native"
            verdict = native / "verifier-input.json"
            verdict.unlink()
        self.reject("fixture-only")

    def test_substituted_bundle_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][0]["bundle_digest"] = "0" * 64
        self.evidence.save_manifest(manifest)
        self.reject("bundle digest")

    def test_missing_command_exit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["command_exits"] = manifest["command_exits"][:-1]
        self.evidence.save_manifest(manifest)
        self.reject("command exits")

    def test_nonzero_command_exit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["command_exits"][0]["exit"] = 2
        self.evidence.save_manifest(manifest)
        self.reject("command exits")

    def test_substituted_preserved_artifact_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"][0]["sha256"] = "0" * 64
        self.evidence.save_manifest(manifest)
        self.reject("preserved artifact")

    def test_missing_preserved_artifact_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"].append(
            {"role": "native", "path": "runs/missing/capture.json", "sha256": sha256(b"x")}
        )
        self.evidence.save_manifest(manifest)
        self.reject("preserved artifact")

    def test_control_drift_against_run_control_is_rejected(self) -> None:
        family = FAMILIES[0]
        control = self.evidence.root / "runs" / family / "run-control.json"
        value = json.loads(control.read_text(encoding="utf-8"))
        value["planned_trials"][0]["subject"] = "someone-else"
        write_json(control, value)
        self.reject("run-control")

    def test_output_limit_claim_without_sealed_controls_is_rejected(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        path = entry["run"] / "trials" / entry["opi"] / "bundle/artifacts/control/experiment.json"
        experiment = json.loads(path.read_text())
        del experiment["model_controls"]
        self.evidence.rewrite_sealed_experiment(FAMILIES[0], experiment)
        self.reject("output-control")

    def test_numeric_output_control_drift_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["controls"]["connection_controls"]["model_controls"]["max_output_tokens"] = 8192
        self.evidence.save_manifest(manifest)
        self.reject("output-control")

    def test_output_control_missing_wire_or_wrong_client_is_rejected(self) -> None:
        for case in (
            "missing", "wire", "binary", "package", "projection",
            "opi-context", "pi-context", "opi-reasoning", "pi-reasoning", "pi-sampling",
            "opi-output-float", "pi-output-float", "opi-context-float", "pi-context-float",
            "opi-output-bool", "pi-output-bool",
        ):
            with self.subTest(case=case):
                self.evidence = EvidenceRoot(self.base / case)
                entry = self.evidence.runs[FAMILIES[0]]
                product = "pi" if case.startswith("pi-") else "opi"
                bundle = entry["run"] / "trials" / entry[product] / "bundle"
                path = bundle / "artifacts/control/output-control.json"
                if case == "missing":
                    path.unlink()
                else:
                    snapshot = json.loads(path.read_text())
                    proof = json.loads(snapshot["evidence"])
                    if case == "wire":
                        snapshot["requests"] = snapshot["requests"].replace("16384", "8192")
                        proof["requests"]["sha256"] = sha256(snapshot["requests"].encode())
                    elif case == "binary":
                        proof["executable_sha256"] = "f" * 64
                    elif case == "package":
                        proof["runtime_package_manifest_sha256"] = "f" * 64
                    elif case.endswith(("float", "bool")):
                        context = "context" in case
                        value = True if case.endswith("bool") else float(1000000 if context else 16384)
                        if product == "opi":
                            field = "context_window" if context else "max_output_tokens"
                            number = 1000000 if context else 16384
                            snapshot["config"] = snapshot["config"].replace(
                                f"{field} = {number}", f"{field} = {json.dumps(value)}"
                            )
                        else:
                            config = json.loads(snapshot["config"])
                            model = config["providers"]["scripted"]["models"][0]
                            model["contextWindow" if context else "maxTokens"] = value
                            snapshot["config"] = json.dumps(config)
                        proof["config"]["sha256"] = sha256(snapshot["config"].encode())
                    elif case.endswith("context"):
                        snapshot["config"] = snapshot["config"].replace("1000000", "4096")
                        proof["config"]["sha256"] = sha256(snapshot["config"].encode())
                    elif case == "opi-reasoning":
                        snapshot["config"] += 'supports_thinking = true\nthinking_level_map = { high = "high" }\n'
                        proof["config"]["sha256"] = sha256(snapshot["config"].encode())
                    elif case in ("pi-reasoning", "pi-sampling"):
                        config = json.loads(snapshot["config"])
                        model = config["providers"]["scripted"]["models"][0]
                        if case == "pi-reasoning":
                            model.update(reasoning=True, thinkingLevelMap={"high": "high"})
                        else:
                            model["samplingParams"] = {"temperature": 0.25, "top_p": 0.9}
                        snapshot["config"] = json.dumps(config)
                        proof["config"]["sha256"] = sha256(snapshot["config"].encode())
                    else:
                        snapshot["config"] = snapshot["config"].replace("16384", "8192")
                        proof["config"]["sha256"] = sha256(snapshot["config"].encode())
                    snapshot["evidence"] = json.dumps(proof)
                    snapshot["evidence_sha256"] = sha256(snapshot["evidence"].encode())
                    write_json(path, snapshot)
                    material_path = entry["run"] / "material.json"
                    material = json.loads(material_path.read_text())
                    material["agents"][product]["output_control_evidence"]["sha256"] = snapshot["evidence_sha256"]
                    write_json(material_path, material)
                    manifest_path = bundle / "manifest.json"
                    sealed = json.loads(manifest_path.read_text())
                    sealed["entries"]["control/output-control.json"]["digest"] = sha256(path.read_bytes())
                    write_json(manifest_path, sealed)
                self.evidence._write_manifest()
                self.reject("output-control")

    def test_swapped_agent_claims_are_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][0]["agent"], manifest["trials"][1]["agent"] = "pi", "opi"
        self.evidence.save_manifest(manifest)
        self.reject("run-control")

    def test_ambiguous_or_wrong_sealed_subject_bindings_are_rejected(self) -> None:
        for case in (
            "swapped-products", "duplicate-subject", "missing-subject",
            "swapped-trials", "duplicate-trial", "missing-trial",
        ):
            with self.subTest(case=case):
                self.evidence = EvidenceRoot(self.base / case)
                family = FAMILIES[0]
                entry = self.evidence.runs[family]
                path = (
                    entry["run"] / "trials" / entry["opi"]
                    / "bundle/artifacts/control/experiment.json"
                )
                experiment = json.loads(path.read_text())
                if case == "swapped-products":
                    experiment["subjects"][0]["product"] = "pi"
                    experiment["subjects"][1]["product"] = "opi"
                elif case == "duplicate-subject":
                    experiment["subjects"].append(experiment["subjects"][0].copy())
                elif case == "missing-subject":
                    experiment["subjects"].pop(0)
                elif case == "swapped-trials":
                    experiment["trials"][0]["subject"] = "baseline-pi"
                    experiment["trials"][1]["subject"] = "candidate-opi"
                elif case == "duplicate-trial":
                    experiment["trials"].append(experiment["trials"][0].copy())
                else:
                    experiment["trials"].pop(0)
                self.evidence.rewrite_sealed_experiment(family, experiment)
                self.reject("run-control")

    def test_duplicate_planned_trial_is_rejected(self) -> None:
        path = self.evidence.runs[FAMILIES[0]]["run"] / "run-control.json"
        control = json.loads(path.read_text())
        control["planned_trials"].append(control["planned_trials"][0].copy())
        write_json(path, control)
        self.reject("run-control")

    def test_run_control_identity_must_match_sealed_experiment(self) -> None:
        for field in ("experiment", "manifest_digest"):
            for missing in (False, True):
                with self.subTest(field=field, missing=missing):
                    self.evidence = EvidenceRoot(self.base / f"{field}-{missing}")
                    path = self.evidence.runs[FAMILIES[0]]["run"] / "run-control.json"
                    control = json.loads(path.read_text())
                    if missing:
                        del control[field]
                    else:
                        control[field] = "substituted"
                    write_json(path, control)
                    self.reject("run-control")

    def test_sealed_experiment_bytes_must_match_bundle_entry(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        path = (
            entry["run"] / "trials" / entry["opi"]
            / "bundle/artifacts/control/experiment.json"
        )
        path.write_bytes(path.read_bytes() + b"\n")
        self.reject("experiment digest")

    def test_missing_sealed_experiment_is_rejected(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        path = (
            entry["run"] / "trials" / entry["opi"]
            / "bundle/artifacts/control/experiment.json"
        )
        path.unlink()
        self.reject("run-control")

    # -- scoped capture view --------------------------------------------------

    def test_empty_scoped_view_is_rejected_even_with_generic_audit_success(self) -> None:
        view = self.evidence.root / "opi-artifact-audit"
        for child in list(view.iterdir()):
            if child.is_dir():
                import shutil

                shutil.rmtree(child)
        write_json(view / "generic-audit-summary.json", {"result": "success"})
        self.reject("empty")

    def test_missing_scoped_view_is_rejected(self) -> None:
        import shutil

        shutil.rmtree(self.evidence.root / "opi-artifact-audit")
        self.reject("opi-artifact-audit")

    def test_substituted_capture_bytes_are_rejected(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        view = self.evidence.root / "opi-artifact-audit" / entry["opi"]
        log = view / "agent-stdout.log"
        log.write_bytes(log.read_bytes() + b"tampered")
        self.reject("byte-bound")

    def test_scoped_view_with_pi_capture_is_not_exhaustive(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        pi_native = entry["run"] / "trials" / entry["pi"] / "bundle" / "artifacts" / "native"
        view = self.evidence.root / "opi-artifact-audit" / entry["pi"]
        view.mkdir()
        for path in sorted(pi_native.iterdir()):
            (view / path.name).write_bytes(path.read_bytes())
        self.reject("exhaustive")

    def test_scoped_view_extra_file_with_non_opi_commit_is_rejected(self) -> None:
        view = self.evidence.root / "opi-artifact-audit"
        (view / "notes.txt").write_text(f"built from {PI_COMMIT}\n", encoding="utf-8")
        self.reject("commit reference")

    # -- upstream identity validation -----------------------------------------

    def test_valid_upstream_identities_are_accepted(self) -> None:
        self.accept()

    def test_mismatched_repository_for_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["pi"]["repository_url"] = HARBOR_REPO
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_unknown_upstream_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["pier"]["commit"] = "d" * 40
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_malformed_upstream_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["harbor"]["commit"] = "not-a-commit"
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_identity_cache_without_repository_is_rejected(self) -> None:
        cache = self.evidence.root / "upstream-identity-cache.json"
        value = json.loads(cache.read_text(encoding="utf-8"))
        del value["repositories"][PIER_REPO]
        write_json(cache, value)
        self.reject("upstream identity")


if __name__ == "__main__":
    unittest.main()
