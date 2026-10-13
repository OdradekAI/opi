#!/usr/bin/env python3
"""Deterministic regressions for the offline local-acceptance verifier.

The verifier reads one supplied redacted evidence root and never launches an
Agent, accesses credentials, or contacts a provider. These tests exercise its
structural logic and its Companion invocation contract with a recording stub;
the Rust test in crates/opi-eval/tests/native_driver.rs verifies that fixture
evidence is rejected. These unit positives do not establish live acceptance.
"""

from __future__ import annotations

import copy
import hashlib
import json
import os
import shlex
import shutil
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
HARBOR_REPO = "https://github.com/harbor-framework/harbor"
PIER_REPO = "https://github.com/datacurve-ai/pier"
PI_COMMIT = "a" * 40
HARBOR_COMMIT = "b" * 40
PIER_COMMIT = "c" * 40
PROBE_BASE_URL = "http://127.0.0.1:48127"
CONNECTION = {"id": "primary", "provider": "zhipu", "api": "anthropic-messages", "base_url": "https://open.bigmodel.cn/api/anthropic", "model": "glm-5.3-flash"}
MODEL_CONTROLS = {"temperature": "omitted", "top_p": "omitted", "max_output_tokens": 16384, "context_window": 1000000, "reasoning": "omitted", "required": ["max_output_tokens"]}


def synthetic_output_control(product, experiment, opi_turn_budget=None):
    """Structural fixture only; real clients run in the ignored Rust matrix."""
    limit = experiment["model_controls"].get("max_output_tokens", 16384)
    if product == "opi":
        config = f'''[defaults]
max_output_tokens = {limit}
[providers.custom.zhipu]
name = "opi-eval zhipu"
base_url = "http://127.0.0.1:48127"
api = "anthropic-messages"
api_key_env = "OPI_EVAL_SCRIPTED_KEY"
auth_scheme = "api-key"
[[providers.custom.zhipu.models]]
id = "glm-5.3-flash"
display_name = "opi-eval zhipu"
max_output_tokens = {limit}
context_window = 1000000
'''
    else:
        config = json.dumps({"providers": {"zhipu": {"baseUrl": PROBE_BASE_URL, "api": CONNECTION["api"], "apiKey": "dummy-scripted-credential", "models": [{"id": "glm-5.3-flash", "maxTokens": limit, "contextWindow": 1000000}]}}})
    if product == "opi" and opi_turn_budget is not None:
        config = config.replace("[defaults]\n", f"[defaults]\nmax_iterations = {opi_turn_budget}\n")
    requests = "".join(json.dumps({"schema": "opi-eval-scripted-provider-log/1", "seq": seq, "status": 200, "method": "POST", "path": "/v1/messages", "model": "glm-5.3-flash", "output_limits": {"max_tokens": limit}, "request_sha256": "d" * 64}) + "\n" for seq in (1, 2))
    proof = {"schema": "opi-eval-output-control/1", "product": product, "executable_sha256": "e" * 64, "runtime_package_manifest_sha256": None, "connection": {key: CONNECTION[key] for key in ("provider", "api", "model")}, "model_controls": experiment["model_controls"], "probe_base_url": PROBE_BASE_URL, "exit_code": 0, "config": {"path": "synthetic-config", "sha256": sha256(config.encode())}, "requests": {"path": "synthetic-requests", "sha256": sha256(requests.encode())}}
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
        self, base: Path, subject_ids: tuple[str, str] = ("candidate-opi", "baseline-pi"),
        opi_turn_budget: str | None = None,
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
        self.opi_turn_budget = opi_turn_budget
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
        lock = {
            "schema": "fixture-static-lock",
            "lock_id": f"fixture-lock-{family}",
            "subjects": [
                {
                    "id": "pi",
                    "kind": "agent-source",
                    "repository_url": PI_REPO,
                    "commit": PI_COMMIT,
                }
            ],
            "tools": [
                {"id": "harbor", "commit": HARBOR_COMMIT},
                {"id": "pier", "commit": PIER_COMMIT},
            ],
        }
        lock_path = run / "static-lock.json"
        write_json(lock_path, lock)
        lock_sha256 = sha256(lock_path.read_bytes())
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
            "execution": {"concurrency": 2, "mode": "live"},
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
        material = {
            "schema": "preserved-material",
            "agents": {},
            "static_lock": {"path": str(lock_path), "sha256": lock_sha256},
        }
        for trial, product in ((opi_id, "opi"), (pi_id, "pi")):
            bundle = run / "trials" / trial / "bundle"
            native = bundle / "artifacts" / "native"
            write_json(
                native / "verifier-input.json",
                {"environment_mode": "separate", "collected": []},
            )
            message = {"role": "assistant", **{key: CONNECTION[key] for key in ("provider", "api", "model")}}
            if product == "opi":
                message["api"] = "Anthropic"
            events = (
                [{"type": "MessageEnd", "message": message}, {"type": "ToolExecutionStart", "tool_call_id": "call"}, {"type": "ToolExecutionEnd", "tool_call_id": "call"}, {"type": "AgentEnd", "messages": [message]}]
                if product == "opi" else
                [{"type": "message_end", "message": message}, {"type": "tool_execution_start", "toolCallId": "call"}, {"type": "tool_execution_end", "toolCallId": "call"}, {"type": "agent_end", "messages": [message]}]
            )
            rows = [{"type": "Agent", "event": event} for event in events] if product == "opi" else events
            stdout = "".join(json.dumps(row) + "\n" for row in rows).encode()
            (native / "agent-stdout.log").write_bytes(stdout)
            (native / "agent-stderr.log").write_bytes(b"")
            preflight_stdout = f"{product} fixture-version\n".encode()
            (native / "runtime-preflight-stdout.log").write_bytes(preflight_stdout)
            (native / "runtime-preflight-stderr.log").write_bytes(b"")
            write_json(native / "agent-terminal.json", {
                "schema": "opi-eval-native-agent-terminal/2", "trial": trial,
                "product": product, "agent_executable_sha256": "e" * 64,
                "runtime_preflight": {
                    "kind": "version", "status": "passed", "exit_code": 0,
                    "agent_executable_sha256": "e" * 64,
                    "runtime_package_manifest_sha256": None,
                    "runtime_package_entrypoint": None,
                    "command_sha256": sha256((shlex.quote(f"/opi-eval-agent/{product}/agent") + " --version").encode()),
                    "stdout_sha256": sha256(preflight_stdout), "stderr_sha256": sha256(b""),
                    "stdout_complete": True, "stderr_complete": True,
                    "stdout_truncated": False, "stderr_truncated": False, "reason": None,
                },
                "task_dispatched": True, "exit_code": 0, "timed_out": False, "failure_kind": None,
                "delivered": [],
                "stdout_truncated": False, "stderr_truncated": False,
                "stdout_sha256": sha256(stdout), "stderr_sha256": sha256(b""),
            })
            write_json(native / "trial-result.json", {
                "agent_info": {"model_info": {"name": "zhipu:glm-5.3-flash"}},
                "exception_info": None, "verifier_result": {"rewards": {"reward": 0}},
            })
            write_json(
                bundle / "intent.json",
                {"schema": "opi-eval-intent/1", "product": product},
            )
            write_json(
                bundle / "artifacts/control/experiment.json",
                experiment,
            )
            snapshot = synthetic_output_control(product, experiment, self.opi_turn_budget)
            wire_path = bundle / "artifacts/control/output-control.json"
            write_json(wire_path, snapshot)
            material["agents"][product] = {"executable": {"sha256": "e" * 64}, "output_control_evidence": {"sha256": snapshot["evidence_sha256"]}}
            write_json(
                bundle / "artifacts/control/settlement.json",
                {
                    "schema": "opi-eval-trial-settlement/1", "trial": trial,
                    "graded": True, "agent_failure": False, "timed_out": False,
                    "failure_kind": None, "boundary": None,
                },
            )
            integrity_path = bundle / "artifacts/control/integrity.json"
            write_json(
                integrity_path,
                {
                    "schema": "fixture-integrity",
                    "upstream_identity": f"{family}@{lock_sha256}",
                },
            )
            write_json(
                bundle / "manifest.json",
                {
                    "schema": "opi-eval-bundle-manifest/1",
                    "entries": {
                        **{f"native/{path.name}": {"digest": sha256(path.read_bytes())} for path in native.iterdir()},
                        "control/output-control.json": {"digest": sha256(wire_path.read_bytes())},
                        "control/settlement.json": {
                            "digest": sha256(
                                (bundle / "artifacts/control/settlement.json").read_bytes()
                            )
                        },
                        "control/integrity.json": {
                            "digest": sha256(integrity_path.read_bytes())
                        },
                        "control/experiment.json": {
                            "digest": sha256(
                                (bundle / "artifacts/control/experiment.json").read_bytes()
                            )
                        }
                    },
                },
            )
        write_json(run / "material.json", material)
        write_json(
            run / "run-report.json",
            {
                "schema": "opi-eval-run-report/1",
                "outcome": "incomplete",
                "trials": [
                    {
                        "id": opi_id,
                        "status": "sealed",
                        "verifier": {"completion": "verified-native"},
                    },
                    {
                        "id": pi_id,
                        "status": "sealed",
                        "verifier": {"completion": "verified-native"},
                    },
                ],
            },
        )

    def rewrite_sealed_family_controls(self, family: str, max_output_tokens: int) -> None:
        """Rebind one family's sealed experiment, wire snapshots and digests."""
        run = self.runs[family]["run"]
        path = run / "trials" / self.runs[family]["pi"] / "bundle/artifacts/control/experiment.json"
        experiment = json.loads(path.read_text())
        experiment["model_controls"] = dict(experiment["model_controls"])
        experiment["model_controls"]["max_output_tokens"] = max_output_tokens
        for product in ("pi", "opi"):
            trial = self.runs[family][product]
            bundle = run / "trials" / trial / "bundle"
            wire = bundle / "artifacts/control/output-control.json"
            snapshot = synthetic_output_control(product, experiment)
            write_json(wire, snapshot)
            sealed = json.loads((bundle / "manifest.json").read_text())
            sealed["entries"]["control/output-control.json"]["digest"] = sha256(wire.read_bytes())
            write_json(bundle / "manifest.json", sealed)
            material = json.loads((run / "material.json").read_text())
            material["agents"][product]["output_control_evidence"]["sha256"] = snapshot["evidence_sha256"]
            write_json(run / "material.json", material)
        self.rewrite_sealed_experiment(family, experiment)

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
            manifest["preserved"].append(
                {
                    "role": "static-lock",
                    "path": f"runs/{family}/static-lock.json",
                    "sha256": sha256((entry["run"] / "static-lock.json").read_bytes()),
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

    def verify(self, label: str | None = None) -> subprocess.CompletedProcess:
        suffix = f"-{label}" if label is not None else ""
        out = self.outside / f"report-{id(self):x}{suffix}"
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
        result = self.evidence.verify(label=fragment.replace(" ", "-")[:40])
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

    def test_synthetic_structured_root_is_accepted(self) -> None:
        report = self.accept()
        self.assertEqual(report["schema"], "opi-eval-local-acceptance-verification/1")
        self.assertEqual(len(report["trials"]), 6)
        out_root = self.evidence.outside
        written = [path.name for path in sorted(out_root.iterdir())]
        self.assertEqual(written, [f"report-{id(self.evidence):x}"])

    def test_valid_native_task_failures_count_as_graded_trials(self) -> None:
        # P19-AUD-007: a sealed graded settlement admits benchmark-valid
        # Agent non-zero exits, task timeouts and clean agent-side
        # non-success terminals (for example a pi `max_tokens` stop the
        # Companion itself grades natively) - including a timeout
        # tail without a final AgentEnd after the completed provider/tool
        # loop and the matching native task exception - as eligible
        # execution trials. Invalid provider/preflight shapes stay
        # ineligible.
        for case, exception in (
            ("graded-nonzero", "NonZeroAgentExitCodeError"),
            ("graded-timeout", "AgentTimeoutError"),
            ("graded-partial-timeout", "AgentTimeoutError"),
            ("graded-outcome-not-success", None),
        ):
            with self.subTest(case=case):
                self.evidence = EvidenceRoot(self.base / f"accepted-{case}")
                entry = self.evidence.runs[FAMILIES[0]]
                for product in ("opi", "pi"):
                    bundle = entry["run"] / "trials" / entry[product] / "bundle"
                    native = bundle / "artifacts/native"
                    terminal_path = native / "agent-terminal.json"
                    terminal = json.loads(terminal_path.read_text())
                    if case == "graded-nonzero":
                        terminal.update(
                            {"exit_code": 1, "failure_kind": "agent-non-zero-exit"}
                        )
                    elif case == "graded-outcome-not-success":
                        terminal.update(
                            {"exit_code": 0, "failure_kind": "agent-outcome-not-success"}
                        )
                    else:
                        terminal.update(
                            {"exit_code": 124, "timed_out": True, "failure_kind": "agent-timeout"}
                        )
                    if case == "graded-partial-timeout":
                        stdout_path = native / "agent-stdout.log"
                        rows = [
                            json.loads(line)
                            for line in stdout_path.read_text().splitlines()
                        ]
                        rows = [
                            row
                            for row in rows
                            if (row.get("event", row)).get("type")
                            not in ("AgentEnd", "agent_end")
                        ]
                        stdout_path.write_text(
                            "".join(json.dumps(row) + "\n" for row in rows)
                        )
                        terminal["stdout_sha256"] = sha256(stdout_path.read_bytes())
                    write_json(terminal_path, terminal)
                    result_path = native / "trial-result.json"
                    result = json.loads(result_path.read_text())
                    result["exception_info"] = (
                        {"exception_type": exception} if exception else None
                    )
                    write_json(result_path, result)
                    settlement_path = bundle / "artifacts/control/settlement.json"
                    write_json(
                        settlement_path,
                        {
                            "schema": "opi-eval-trial-settlement/1",
                            "trial": entry[product],
                            "graded": True,
                            "agent_failure": True,
                            "timed_out": terminal["timed_out"],
                            "failure_kind": None,
                            "boundary": None,
                        },
                    )
                    manifest_path = bundle / "manifest.json"
                    manifest = json.loads(manifest_path.read_text())
                    for path in native.iterdir():
                        manifest["entries"][f"native/{path.name}"] = {
                            "digest": sha256(path.read_bytes())
                        }
                    manifest["entries"]["control/settlement.json"] = {
                        "digest": sha256(settlement_path.read_bytes())
                    }
                    write_json(manifest_path, manifest)
                for family, entry in self.evidence.runs.items():
                    native = (
                        entry["run"]
                        / "trials"
                        / entry["opi"]
                        / "bundle/artifacts/native"
                    )
                    for path in native.iterdir():
                        (
                            self.evidence.root
                            / "opi-artifact-audit"
                            / entry["opi"]
                            / path.name
                        ).write_bytes(path.read_bytes())
                self.evidence._write_manifest()
                self.accept()
        # Provider and preflight failures never become eligible.
        for case, changes in (
            ("invalid-provider", {"exit_code": 3, "failure_kind": "agent-provider-failure"}),
            (
                "invalid-preflight",
                {
                    "task_dispatched": False,
                    "exit_code": None,
                    "failure_kind": "native-runtime-preflight-refused",
                },
            ),
        ):
            with self.subTest(case=case):
                self.evidence = EvidenceRoot(self.base / f"rejected-{case}")
                entry = self.evidence.runs[FAMILIES[0]]
                for product in ("opi", "pi"):
                    bundle = entry["run"] / "trials" / entry[product] / "bundle"
                    native = bundle / "artifacts/native"
                    terminal_path = native / "agent-terminal.json"
                    terminal = json.loads(terminal_path.read_text())
                    terminal.update(changes)
                    write_json(terminal_path, terminal)
                    result_path = native / "trial-result.json"
                    result = json.loads(result_path.read_text())
                    result["exception_info"] = {
                        "exception_type": "ApiError"
                        if case == "invalid-provider"
                        else "AgentSetupError"
                    }
                    write_json(result_path, result)
                    settlement_path = bundle / "artifacts/control/settlement.json"
                    write_json(
                        settlement_path,
                        {
                            "schema": "opi-eval-trial-settlement/1",
                            "trial": entry[product],
                            "graded": False,
                            "agent_failure": False,
                            "timed_out": bool(changes.get("timed_out")),
                            "failure_kind": changes["failure_kind"],
                            "boundary": "agent-process",
                        },
                    )
                    manifest_path = bundle / "manifest.json"
                    manifest = json.loads(manifest_path.read_text())
                    for path in native.iterdir():
                        manifest["entries"][f"native/{path.name}"] = {
                            "digest": sha256(path.read_bytes())
                        }
                    manifest["entries"]["control/settlement.json"] = {
                        "digest": sha256(settlement_path.read_bytes())
                    }
                    write_json(manifest_path, manifest)
                self.evidence._write_manifest()
                self.reject("agent evidence")

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

    def test_first_connection_and_live_mode_are_required(self) -> None:
        for field, value in (
            ("mode", None), ("mode", "native-fixture"), ("provider", "scripted"),
            ("api", "openai-completions"), ("base_url", PROBE_BASE_URL),
            ("model", "other"),
        ):
            with self.subTest(field=field, value=value):
                self.evidence = EvidenceRoot(self.base / f"connection-{field}-{value}")
                family = FAMILIES[0]
                entry = self.evidence.runs[family]
                path = entry["run"] / "trials" / entry["opi"] / "bundle/artifacts/control/experiment.json"
                experiment = json.loads(path.read_text())
                if field == "mode":
                    if value is None:
                        experiment["execution"].pop("mode")
                    else:
                        experiment["execution"]["mode"] = value
                else:
                    experiment["connections"][0][field] = value
                self.evidence.rewrite_sealed_experiment(family, experiment)
                self.reject("first connection")

    def test_live_agent_observations_cannot_be_substituted(self) -> None:
        for product in ("opi", "pi"):
            for case in ("missing", "terminal", "stream", "model", "provider", "tools", "api", "unknown-api", "wrong-api"):
                with self.subTest(product=product, case=case):
                    self.evidence = EvidenceRoot(self.base / f"observed-{product}-{case}")
                    entry = self.evidence.runs[FAMILIES[0]]
                    bundle = entry["run"] / "trials" / entry[product] / "bundle"
                    native = bundle / "artifacts/native"
                    terminal_path = native / "agent-terminal.json"
                    terminal = json.loads(terminal_path.read_text())
                    if case == "missing":
                        terminal_path.unlink()
                    elif case == "terminal":
                        terminal["exit_code"] = 3
                        write_json(terminal_path, terminal)
                    elif case == "model":
                        path = native / "trial-result.json"
                        result = json.loads(path.read_text())
                        result["agent_info"]["model_info"]["name"] = "scripted:eval"
                        write_json(path, result)
                    else:
                        path = native / "agent-stdout.log"
                        raw = path.read_text()
                        if case == "provider":
                            raw = raw.replace('"zhipu"', '"scripted"')
                        elif case in ("api", "unknown-api", "wrong-api"):
                            expected = "Anthropic" if product == "opi" else "anthropic-messages"
                            replacement = {
                                "api": "anthropic-messages" if product == "opi" else "Anthropic",
                                "unknown-api": "invented",
                                "wrong-api": "OpenAi" if product == "opi" else "openai-completions",
                            }[case]
                            raw = raw.replace(json.dumps(expected), json.dumps(replacement))
                        elif case == "tools":
                            raw = "\n".join(line for line in raw.splitlines() if "oolExecution" not in line and "tool_execution" not in line) + "\n"
                        else:
                            raw += "{}\n"
                        path.write_text(raw)
                        if case != "stream":
                            terminal["stdout_sha256"] = sha256(path.read_bytes())
                            write_json(terminal_path, terminal)
                    manifest_path = bundle / "manifest.json"
                    manifest = json.loads(manifest_path.read_text())
                    for path in native.iterdir():
                        manifest["entries"][f"native/{path.name}"] = {"digest": sha256(path.read_bytes())}
                    write_json(manifest_path, manifest)
                    self.evidence._write_manifest()
                    self.reject("agent evidence")

    def test_runtime_preflight_is_required_and_bound(self) -> None:
        cases = (
            "legacy", "missing", "unknown", "failed", "trial", "product",
            "terminal-executable", "preflight-executable", "command", "log",
            "missing-log", "incomplete", "truncated", "unreadable", "nul", "empty",
            "exit-bool", "reason", "dispatched", "package", "entrypoint",
            "missing-field",
        )
        for product in ("opi", "pi"):
            for case in cases:
                with self.subTest(product=product, case=case):
                    self.evidence = EvidenceRoot(self.base / f"preflight-{product}-{case}")
                    entry = self.evidence.runs[FAMILIES[0]]
                    bundle = entry["run"] / "trials" / entry[product] / "bundle"
                    native = bundle / "artifacts/native"
                    terminal_path = native / "agent-terminal.json"
                    terminal = json.loads(terminal_path.read_text())
                    proof = terminal["runtime_preflight"]
                    if case == "legacy":
                        terminal["schema"] = "opi-eval-native-agent-terminal/1"
                    elif case == "missing":
                        del terminal["runtime_preflight"]
                    elif case in ("unknown", "failed"):
                        proof.update(status=case, exit_code=None if case == "unknown" else 127)
                        terminal.update(task_dispatched=False, exit_code=None, delivered=[])
                    elif case == "trial":
                        terminal["trial"] = "stale-trial"
                    elif case == "product":
                        terminal["product"] = "pi" if product == "opi" else "opi"
                    elif case == "terminal-executable":
                        terminal["agent_executable_sha256"] = "f" * 64
                    elif case == "preflight-executable":
                        proof["agent_executable_sha256"] = "f" * 64
                    elif case == "command":
                        proof["command_sha256"] = "f" * 64
                    elif case in ("log", "unreadable", "nul", "empty"):
                        data = {"log": b"substituted\n", "unreadable": b"\xff", "nul": b"fixture\x00version\n", "empty": b""}[case]
                        (native / "runtime-preflight-stdout.log").write_bytes(data)
                        if case != "log":
                            proof["stdout_sha256"] = sha256(data)
                    elif case == "missing-log":
                        (native / "runtime-preflight-stderr.log").unlink()
                    elif case == "incomplete":
                        proof["stderr_complete"] = False
                    elif case == "truncated":
                        proof["stdout_truncated"] = True
                    elif case == "exit-bool":
                        proof["exit_code"] = False
                    elif case == "reason":
                        proof["reason"] = "transport"
                    elif case == "dispatched":
                        terminal["task_dispatched"] = False
                    elif case == "package":
                        proof["runtime_package_manifest_sha256"] = "f" * 64
                    elif case == "entrypoint":
                        proof["runtime_package_entrypoint"] = "agent"
                    else:
                        del proof["stderr_complete"]
                    write_json(terminal_path, terminal)
                    manifest_path = bundle / "manifest.json"
                    manifest = json.loads(manifest_path.read_text())
                    for path in native.iterdir():
                        manifest["entries"][f"native/{path.name}"] = {"digest": sha256(path.read_bytes())}
                    write_json(manifest_path, manifest)
                    self.evidence._write_manifest()
                    if case == "nul" and product == "opi":
                        view = self.evidence.root / "opi-artifact-audit" / entry[product]
                        for path in native.iterdir():
                            (view / path.name).write_bytes(path.read_bytes())
                    self.reject("agent evidence")

    def test_packaged_runtime_preflight_binds_entrypoint_and_manifest(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        bundle = entry["run"] / "trials" / entry["pi"] / "bundle"
        package = {"manifest_sha256": "f" * 64, "entrypoint": "bin/pi wrapper"}
        material_path = entry["run"] / "material.json"
        material = json.loads(material_path.read_text())
        material["agents"]["pi"]["runtime_package"] = package
        snapshot_path = bundle / "artifacts/control/output-control.json"
        snapshot = json.loads(snapshot_path.read_text())
        output_proof = json.loads(snapshot["evidence"])
        output_proof.update(runtime_package_manifest_sha256=package["manifest_sha256"],
                            runtime_package_entrypoint=package["entrypoint"])
        snapshot["evidence"] = json.dumps(output_proof)
        snapshot["evidence_sha256"] = sha256(snapshot["evidence"].encode())
        material["agents"]["pi"]["output_control_evidence"]["sha256"] = snapshot["evidence_sha256"]
        write_json(material_path, material)
        write_json(snapshot_path, snapshot)
        terminal_path = bundle / "artifacts/native/agent-terminal.json"
        terminal = json.loads(terminal_path.read_text())
        proof = terminal["runtime_preflight"]
        proof.update(runtime_package_manifest_sha256=package["manifest_sha256"],
                     runtime_package_entrypoint=package["entrypoint"],
                     command_sha256=sha256((shlex.quote("/opi-eval-agent/pi/runtime/" + package["entrypoint"]) + " --version").encode()))
        write_json(terminal_path, terminal)
        manifest_path = bundle / "manifest.json"
        manifest = json.loads(manifest_path.read_text())
        for key in ("control/output-control.json", "native/agent-terminal.json"):
            manifest["entries"][key] = {"digest": sha256((bundle / "artifacts" / key).read_bytes())}
        write_json(manifest_path, manifest)
        self.evidence._write_manifest()
        self.accept()
        proof["runtime_package_entrypoint"] = "bin/other"
        write_json(terminal_path, terminal)
        manifest["entries"]["native/agent-terminal.json"]["digest"] = sha256(terminal_path.read_bytes())
        write_json(manifest_path, manifest)
        self.evidence._write_manifest()
        self.evidence.outside = self.base / "substituted-package-out"
        self.reject("agent evidence")

    def test_runtime_preflight_capture_links_are_rejected(self) -> None:
        for product in ("opi", "pi"):
            with self.subTest(product=product):
                self.evidence = EvidenceRoot(self.base / f"linked-preflight-{product}")
                entry = self.evidence.runs[FAMILIES[0]]
                capture = entry["run"] / "trials" / entry[product] / "bundle/artifacts/native/runtime-preflight-stdout.log"
                target = self.base / f"outside-preflight-{product}.log"
                target.write_bytes(capture.read_bytes())
                capture.unlink()
                capture.symlink_to(target)
                self.reject("agent evidence")

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

    def test_unsafe_trial_ids_reject_before_reads(self) -> None:
        canary = self.base / "escape-canary.json"
        write_json(canary, {"marker": "harmless-canary"})
        pristine = self.evidence.load_manifest()
        for index, bad in enumerate((
            "/escape-canary.json",
            "../../../escape-canary.json",
            "C:\\canary",
            ".",
            "..",
            "nested/id",
            "",
            123,
            None,
        )):
            manifest = copy.deepcopy(pristine)
            victim = next(
                trial
                for trial in manifest["trials"]
                if trial["id"] == self.evidence.runs[FAMILIES[0]]["opi"]
            )
            victim["id"] = bad
            self.evidence.save_manifest(manifest)
            result = self.evidence.verify(label=str(index))
            self.assertEqual(result.returncode, 2, f"expected rejection: {result.stderr}")
            self.assertIn("trial id:", result.stderr)

    def test_run_root_symlink_escape_is_rejected(self) -> None:
        family = FAMILIES[0]
        outside = self.base / "outside-root"
        shutil.copytree(self.evidence.runs[family]["run"], outside, symlinks=True)
        shutil.rmtree(self.evidence.runs[family]["run"])
        (self.evidence.root / "runs" / family).symlink_to(outside)
        self.reject("escapes the evidence root")

    def test_conflicting_family_pair_roots_are_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        family = FAMILIES[0]
        other = FAMILIES[1]
        victim = next(
            trial for trial in manifest["trials"]
            if trial["family"] == family and trial["agent"] == "pi"
        )
        victim["run_root"] = f"runs/{other}"
        self.evidence.save_manifest(manifest)
        self.reject("conflicting roots")

    def test_benign_alternate_family_roots_are_admitted(self) -> None:
        family = FAMILIES[0]
        alternate = self.evidence.root / "custom-evidence" / family
        alternate.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(self.evidence.runs[family]["run"]), str(alternate))
        manifest = self.evidence.load_manifest()
        for trial in manifest["trials"]:
            if trial["family"] == family:
                trial["run_root"] = f"custom-evidence/{family}"
        for entry in manifest["preserved"]:
            entry["path"] = entry["path"].replace(
                f"runs/{family}/", f"custom-evidence/{family}/"
            )
        self.evidence.save_manifest(manifest)
        self.accept()
        invocations = self.evidence.stub_record.read_text(encoding="utf-8").splitlines()
        self.assertIn(f"verify --root {alternate}", invocations)

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

    def test_run_exit_one_with_all_trials_sealed_is_accepted(self) -> None:
        # A paired run whose every trial is sealed and verified natively can
        # settle non-zero through its comparability marker; that is genuine
        # operator evidence, not a command failure.
        manifest = self.evidence.load_manifest()
        for entry in manifest["command_exits"]:
            if entry["command"] == "run":
                entry["exit"] = 1
        self.evidence.save_manifest(manifest)
        self.accept()

    def test_run_exit_one_with_unsealed_trial_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        for entry in manifest["command_exits"]:
            if entry["command"] == "run":
                entry["exit"] = 1
        self.evidence.save_manifest(manifest)
        family = FAMILIES[0]
        path = self.evidence.runs[family]["run"] / "run-report.json"
        report = json.loads(path.read_text())
        report["trials"][0]["status"] = "failed"
        write_json(path, report)
        self.reject("command exits")

    def test_run_exit_one_without_run_report_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        for entry in manifest["command_exits"]:
            if entry["command"] == "run":
                entry["exit"] = 1
        self.evidence.save_manifest(manifest)
        (self.evidence.runs[FAMILIES[0]]["run"] / "run-report.json").unlink()
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

    def test_per_family_control_blocks_accept_and_reject(self) -> None:
        # Primary sources may prescribe one protocol per benchmark family;
        # the per-family declaration must cover exactly the accepted
        # families and match each family's sealed experiment.
        family = FAMILIES[0]
        self.evidence.rewrite_sealed_family_controls(family, 8192)
        manifest = self.evidence.load_manifest()
        shared = manifest["controls"]["connection_controls"]
        block = {
            "execution_concurrency": manifest["controls"]["execution_concurrency"],
            "connection_controls": {
                "connection": dict(shared["connection"]),
                "model_controls": {**shared["model_controls"], "max_output_tokens": 8192},
            },
        }
        manifest["controls"] = {
            "families": {name: {"execution_concurrency": block["execution_concurrency"],
                                "connection_controls": {
                                    "connection": dict(shared["connection"]),
                                    "model_controls": dict(shared["model_controls"]),
                                }} for name in FAMILIES}
        }
        manifest["controls"]["families"][family] = block
        self.evidence.save_manifest(manifest)
        self.accept()
        drifted = self.evidence.load_manifest()
        drifted["controls"]["families"][family]["connection_controls"]["model_controls"]["max_output_tokens"] = 4096
        self.evidence.save_manifest(drifted)
        self.reject("output-control")
        uncovered = self.evidence.load_manifest()
        del uncovered["controls"]["families"][family]
        self.evidence.save_manifest(uncovered)
        self.reject("per-family controls must cover exactly the accepted families")

    def test_sealed_turn_budget_accepts_historical_and_current_projections(self) -> None:
        for index, budget in enumerate((
            None, "4294967295", "50", "0", "-1", "4294967296",
            "true", "4294967295.0", '"4294967295"',
        )):
            with self.subTest(budget=budget):
                self.evidence = EvidenceRoot(self.base / f"turn-{index}", opi_turn_budget=budget)
                if budget in (None, "4294967295"):
                    self.accept()
                else:
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
                            model = config["providers"]["zhipu"]["models"][0]
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
                        model = config["providers"]["zhipu"]["models"][0]
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

    def test_coordinated_source_and_cache_substitution_is_rejected(self) -> None:
        """A coordinated substitution of manifest sources and the operator
        cache must still fail against the lock-derived identity."""
        manifest = self.evidence.load_manifest()
        substituted_pi = "e" * 40
        substituted_harbor = "f" * 40
        substituted_pier = "9" * 40
        manifest["sources"]["pi"]["commit"] = substituted_pi
        manifest["sources"]["harbor"]["commit"] = substituted_harbor
        manifest["sources"]["pier"]["commit"] = substituted_pier
        self.evidence.save_manifest(manifest)
        cache = self.evidence.root / "upstream-identity-cache.json"
        value = json.loads(cache.read_text(encoding="utf-8"))
        value["repositories"][PI_REPO]["commits"][substituted_pi] = {"tree": "4" * 40}
        value["repositories"][HARBOR_REPO]["commits"][substituted_harbor] = {
            "tree": "5" * 40
        }
        value["repositories"][PIER_REPO]["commits"][substituted_pier] = {"tree": "6" * 40}
        write_json(cache, value)
        self.reject("admitted lock identity")

    def test_empty_material_inventory_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["material"] = []
        self.evidence.save_manifest(manifest)
        self.reject("material")

    def test_empty_preserved_inventory_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"] = []
        self.evidence.save_manifest(manifest)
        self.reject("admitted static lock")

    def test_duplicate_material_entry_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["material"].append(dict(manifest["material"][0]))
        self.evidence.save_manifest(manifest)
        self.reject("exactly one")

    def test_duplicate_preserved_path_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"].append(dict(manifest["preserved"][0]))
        self.evidence.save_manifest(manifest)
        self.reject("duplicate path")

    def test_missing_required_preserved_reference_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"] = [
            entry
            for entry in manifest["preserved"]
            if entry.get("role") != "static-lock"
        ]
        self.evidence.save_manifest(manifest)
        self.reject("admitted static lock")

    def test_admitted_lock_digest_tamper_is_rejected(self) -> None:
        lock = self.evidence.runs[FAMILIES[0]]["run"] / "static-lock.json"
        write_json(lock, {"schema": "tampered"})
        self.reject("admitted lock")

    def test_cross_family_lock_binding_is_rejected(self) -> None:
        """One family's sealed integrity must bind its own admitted lock,
        not another family's."""
        first = self.evidence.runs[FAMILIES[0]]["run"]
        second = self.evidence.runs[FAMILIES[1]]["run"]
        other_lock_sha = sha256((second / "static-lock.json").read_bytes())
        self.rebind_upstream_identity(FAMILIES[0], f"{FAMILIES[0]}@{other_lock_sha}")
        self.reject("sealed upstream identity")

    def test_sealed_upstream_identity_drift_is_rejected(self) -> None:
        self.rebind_upstream_identity(
            FAMILIES[2], f"{FAMILIES[2]}@{'0' * 64}"
        )
        self.reject("sealed upstream identity")

    def rebind_upstream_identity(self, family: str, identity: str) -> None:
        """Rewrite one family's sealed integrity with intact digests so the
        identity binding itself is the only drift."""
        entry = self.evidence.runs[family]
        manifest = self.evidence.load_manifest()
        for trial in entry["run"].joinpath("trials").iterdir():
            integrity = trial / "bundle/artifacts/control/integrity.json"
            write_json(integrity, {"schema": "fixture-integrity", "upstream_identity": identity})
            bundle_manifest = trial / "bundle/manifest.json"
            value = json.loads(bundle_manifest.read_text(encoding="utf-8"))
            value["entries"]["control/integrity.json"]["digest"] = sha256(
                integrity.read_bytes()
            )
            write_json(bundle_manifest, value)
            for row in manifest["trials"]:
                if row["id"] == trial.name:
                    row["bundle_digest"] = sha256(bundle_manifest.read_bytes())
        self.evidence.save_manifest(manifest)


if __name__ == "__main__":
    unittest.main()
