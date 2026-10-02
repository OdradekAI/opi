#!/usr/bin/env python3
"""Contract tests for the opi-eval materialization CI verifier.

The verifier pins the committed lock-materialization workflow: its path, its
bytes, every producer script it invokes, every action it uses, and its
manual-authorization contract. These tests exercise the rejection matrix
(missing context, workflow path mismatch, workflow drift, unhashed invoked
producer, mutable action, candidate/workflow contract mismatch) against
hermetic temporary workspaces, and the acceptance path against the real
committed repository files.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify-external-lock-ci.py")
REPO_ROOT = Path(__file__).resolve().parents[3]

WORKFLOW_PATH = ".github/workflows/opi-eval-external-lock-materialization.yml"
MATERIALIZER = "crates/opi-eval/scripts/materialize-external-locks.sh"
CI_VERIFIER = "crates/opi-eval/scripts/verify-external-lock-ci.py"
STATIC_LOCK = "crates/opi-eval/external-locks/static/linux-x86_64.json"

ACTION_COMMIT = "a" * 40
OTHER_ACTION_COMMIT = "b" * 40
WORKFLOW_SHA = "c" * 64
MATERIALIZER_SHA = "d" * 64
CI_VERIFIER_SHA = "e" * 64


def lf_sha256(data: bytes) -> str:
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def valid_workflow_text(trigger: str = "workflow_dispatch") -> str:
    return (
        "name: opi-eval external-lock materialization\n"
        f"on:\n  {trigger}:\n    inputs:\n      candidate_sha:\n        required: true\n"
        "permissions:\n  contents: read\n"
        "concurrency:\n  group: opi-eval-lock-materialization\n"
        "  cancel-in-progress: false\n"
        "jobs:\n  materialize:\n    runs-on: ubuntu-24.04\n    steps:\n"
        f"      - uses: actions/checkout@{ACTION_COMMIT} # v4.2.2\n"
        f"      - run: python3 {CI_VERIFIER} --workflow {WORKFLOW_PATH} --script {MATERIALIZER}\n"
        f"      - run: bash {MATERIALIZER} --candidate-commit \"${{{{ inputs.candidate_sha }}}}\"\n"
    )


def valid_static_lock(
    workflow_sha: str = WORKFLOW_SHA,
    workflow_path: str = WORKFLOW_PATH,
    materializer_sha: str = MATERIALIZER_SHA,
    verifier_sha: str = CI_VERIFIER_SHA,
    schema: str = "opi-eval-external-lock/static/1",
) -> dict:
    return {
        "schema": schema,
        "lock_id": "opi-eval-linux-x86_64",
        "platform": "linux-x86_64",
        "authority": {
            "trigger": "workflow_dispatch",
            "admission": "digest",
            "workflow": {"path": workflow_path, "sha256": workflow_sha},
            "producers": [
                {"path": MATERIALIZER, "role": "materializer", "sha256": materializer_sha},
                {"path": CI_VERIFIER, "role": "verifier", "sha256": verifier_sha},
            ],
            "actions": [
                {"name": "actions/checkout", "version": "v4.2.2", "commit": ACTION_COMMIT}
            ],
        },
    }


class Workspace:
    """A hermetic repository-shaped workspace for one verifier run."""

    def __init__(
        self,
        workflow_text: str | None = None,
        static_lock: dict | None = None,
        extra_files: dict[str, bytes] | None = None,
        workflow_name: str = WORKFLOW_PATH,
    ) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="opi-eval-ci-verify-"))
        workflow_text = (
            valid_workflow_text() if workflow_text is None else workflow_text
        )
        static_lock = valid_static_lock() if static_lock is None else static_lock
        for name, body in {
            MATERIALIZER: b"#!/usr/bin/env bash\nset -euo pipefail\n",
            CI_VERIFIER: b"#!/usr/bin/env python3\n",
            **(extra_files or {}),
        }.items():
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(body)
        # Bind the recorded digests to the actual bytes unless a test overrode
        # them for a drift case.
        workflow_path = self.root / workflow_name
        workflow_path.parent.mkdir(parents=True, exist_ok=True)
        workflow_path.write_text(workflow_text, encoding="utf-8", newline="\n")
        if static_lock["authority"]["workflow"]["sha256"] == WORKFLOW_SHA:
            static_lock["authority"]["workflow"]["sha256"] = lf_sha256(
                workflow_path.read_bytes()
            )
        for producer in static_lock["authority"]["producers"]:
            placeholder = {
                MATERIALIZER: MATERIALIZER_SHA,
                CI_VERIFIER: CI_VERIFIER_SHA,
            }.get(producer["path"])
            if producer["sha256"] == placeholder:
                producer["sha256"] = lf_sha256(
                    (self.root / producer["path"]).read_bytes()
                )
        lock_path = self.root / STATIC_LOCK
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        lock_path.write_text(
            json.dumps(static_lock, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
    def ensure_file(self, name: str, body: bytes) -> None:
        target = self.root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)

    def run(self, *extra_args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--workflow",
                str(self.root / WORKFLOW_PATH),
                "--script",
                str(self.root / MATERIALIZER),
                *extra_args,
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )


def seeded_workspace() -> Workspace:
    return Workspace()


class RejectsMissingContext(unittest.TestCase):
    def test_missing_workflow_argument_rejects(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--script", str(REPO_ROOT / MATERIALIZER)],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)

    def test_missing_script_argument_rejects(self) -> None:
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--workflow", str(REPO_ROOT / WORKFLOW_PATH)],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stderr + result.stdout)


class RejectsWorkflowPathMismatch(unittest.TestCase):
    def test_workflow_at_unpinned_path_rejects(self) -> None:
        ws = Workspace(workflow_name=".github/workflows/other-workflow.yml")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--workflow",
                str(ws.root / ".github/workflows/other-workflow.yml"),
                "--script",
                str(ws.root / MATERIALIZER),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("path", (result.stdout + result.stderr).lower())


class RejectsWorkflowDrift(unittest.TestCase):
    def test_workflow_bytes_drift_rejects(self) -> None:
        ws = Workspace()
        workflow = ws.root / WORKFLOW_PATH
        workflow.write_text(
            workflow.read_text(encoding="utf-8") + "# drifted\n",
            encoding="utf-8",
            newline="\n",
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("drift", (result.stdout + result.stderr).lower())


class RejectsUnhashedInvokedProducer(unittest.TestCase):
    def test_unpinned_script_argument_rejects(self) -> None:
        ws = Workspace()
        rogue = ws.root / "crates/opi-eval/scripts/rogue-producer.sh"
        rogue.write_bytes(b"#!/usr/bin/env bash\n")
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--workflow",
                str(ws.root / WORKFLOW_PATH),
                "--script",
                str(rogue),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())

    def test_producer_bytes_drift_rejects(self) -> None:
        ws = seeded_workspace()
        (ws.root / MATERIALIZER).write_bytes(b"#!/usr/bin/env bash\n# tampered\n")
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())

    def test_workflow_invoking_unpinned_script_rejects(self) -> None:
        ws = Workspace(
            extra_files={"scripts/forgotten-helper.sh": b"#!/usr/bin/env bash\n"}
        )
        workflow = ws.root / WORKFLOW_PATH
        workflow.write_text(
            workflow.read_text(encoding="utf-8")
            + "      - run: bash scripts/forgotten-helper.sh\n",
            encoding="utf-8",
            newline="\n",
        )
        # Re-bind the recorded workflow digest to the tampered bytes; the
        # unpinned invocation must still reject.
        lock_path = ws.root / STATIC_LOCK
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        lock["authority"]["workflow"]["sha256"] = lf_sha256(workflow.read_bytes())
        lock_path.write_text(
            json.dumps(lock, indent=2) + "\n", encoding="utf-8", newline="\n"
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("producer", (result.stdout + result.stderr).lower())


class RejectsMutableAction(unittest.TestCase):
    def test_tag_pinned_action_rejects(self) -> None:
        ws = Workspace(workflow_text=valid_workflow().replace(ACTION_COMMIT, "v4.2.2"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("action", (result.stdout + result.stderr).lower())

    def test_unrecorded_action_commit_rejects(self) -> None:
        ws = Workspace(
            workflow_text=valid_workflow().replace(ACTION_COMMIT, OTHER_ACTION_COMMIT)
        )
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("action", (result.stdout + result.stderr).lower())


def valid_workflow() -> str:
    return valid_workflow_text()


class RejectsContractMismatch(unittest.TestCase):
    def make(self, workflow_text: str) -> Workspace:
        ws = Workspace(workflow_text=workflow_text)
        return ws

    def test_push_trigger_rejects(self) -> None:
        ws = self.make(valid_workflow_text(trigger="push"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("trigger", (result.stdout + result.stderr).lower())

    def test_missing_materializer_invocation_rejects(self) -> None:
        text = valid_workflow().replace(
            f'      - run: bash {MATERIALIZER} --candidate-commit "${{{{ inputs.candidate_sha }}}}"\n',
            "",
        )
        ws = self.make(text)
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("materializer", (result.stdout + result.stderr).lower())

    def test_missing_read_permissions_rejects(self) -> None:
        text = valid_workflow().replace("permissions:\n  contents: read\n", "")
        ws = self.make(text)
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("permission", (result.stdout + result.stderr).lower())

    def test_wrong_static_lock_schema_rejects(self) -> None:
        ws = Workspace(static_lock=valid_static_lock(schema="other/1"))
        result = ws.run()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("schema", (result.stdout + result.stderr).lower())


class AcceptsCommittedContract(unittest.TestCase):
    def test_seeded_workspace_accepts(self) -> None:
        ws = seeded_workspace()
        result = ws.run()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_real_repository_files_accept(self) -> None:
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(REPO_ROOT),
                "--workflow",
                str(REPO_ROOT / WORKFLOW_PATH),
                "--script",
                str(REPO_ROOT / MATERIALIZER),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


# ---------------------------------------------------------------------------
# Default-route delegation: the committed materializer's default mode must
# publish a /2 receipt only through the shared prepare/conformance CLIs
# (D4). Fake tools and synthetic intercepted URL bytes only; no real
# HTTP/Docker/uv/Cargo/Agent/grader run.
# ---------------------------------------------------------------------------

DELEGATION_COMMIT = "a" * 40
DELEGATION_TASK = b'[task]\nname = "fixture"\n[environment]\nallow_internet = true\n'


def _delegation_tools(home: Path, static_sha: str, refuse: str | None, variant: str | None) -> None:
    """Install the fake PATH tools (git/curl/docker/cargo/opi-eval/python3)."""
    tools = home / "bin"
    tools.mkdir(parents=True, exist_ok=True)
    tool = r"""#!/usr/bin/python3
import json, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
args = sys.argv[1:]
context = json.loads(pathlib.Path(__import__("os").environ["PROBE_CONTEXT"]).read_text())
with pathlib.Path(context["calls"]).open("a") as log:
    log.write(json.dumps([name, *args]) + "\n")
if name == "git":
    if args[:1] == ["-C"]:
        args = args[2:]
    if args[0] == "rev-parse":
        print(context["commit"])
    else:
        raise SystemExit("unmocked git: " + repr(args))
elif name == "docker":
    if args[0] == "version":
        print("0.0.synthetic")
    elif args[:3] == ["buildx", "imagetools", "inspect"]:
        import base64
        sys.stdout.buffer.write(base64.b64decode(context["image_manifest"]))
    else:
        raise SystemExit("unmocked docker: " + repr(args))
elif name == "curl":
    raise SystemExit("unmocked curl: " + repr(args))
elif name == "cargo":
    print(json.dumps({"reason": "compiler-artifact", "target": {"name": "opi-eval"},
                      "executable": context["companion"]}))
elif name == "opi-eval":
    if context["refuse"] == args[0]:
        raise SystemExit("synthetic shared-" + args[0] + " refusal")
    if args[0] == "prepare":
        out = pathlib.Path(args[args.index("--out") + 1])
        out.mkdir(parents=True, exist_ok=True)
        (out / "material.json").write_text(json.dumps({
            "schema": "opi-eval-native-material/7", "origin": "local",
            "static_lock": {"sha256": context["static_sha"]},
            "agents": {"opi": {}, "pi": {}},
            "benchmarks": {"terminal-bench-2.1": {"tasks": {"fixture": {}}}},
        }))
        cache = pathlib.Path(args[args.index("--config") + 1]).parent / "cache"
        task = cache / "corpora/terminal-bench-2.1/tasks/fixture"
        task.mkdir(parents=True, exist_ok=True)
        import base64
        (task / "task.toml").write_bytes(base64.b64decode(context["task"]))
        print(json.dumps({"schema": "opi-eval-prepare/1", "origin": "local",
                          "material": str(out / "material.json")}))
    elif args[0] == "conformance":
        root = pathlib.Path(args[args.index("--root") + 1])
        root.mkdir(parents=True, exist_ok=True)
        task_id = "other-task" if context["variant"] == "wrong-task" else "fixture"
        if context["variant"] != "missing-raw":
            (root / "jobs/j1/native-result.json").parent.mkdir(parents=True, exist_ok=True)
            (root / "jobs/j1/native-result.json").write_text(
                json.dumps({"verifier_result": {"rewards": {"reward": 1}}}))
        (root / "ctrf-report.json").write_text(
            json.dumps({"results": {"tests": [{"state": "passed"}]}}))
        print(json.dumps({
            "schema": "opi-eval-conformance-report/1", "suite": "benchmark",
            "adapter": "terminal-bench-2.1", "case": "oracle-witness",
            "exit_state": "exited:0", "outcome": "verified-native",
            "identity": {"admitted_lock_digest": context["static_sha"], "task_id": task_id},
            "reward": {"state": "known", "value": 1, "origin": "native-trial-result"},
            "met": True,
        }))
    else:
        raise SystemExit("unmocked companion: " + repr(args))
else:
    raise SystemExit("unmocked tool: " + name)
"""
    for name in ("git", "curl", "docker", "cargo", "opi-eval"):
        target = tools / name
        target.write_text(tool, encoding="utf-8", newline="\n")
        target.chmod(0o755)
    python_wrapper = r"""#!/usr/bin/python3
import base64, json, os, pathlib, runpy, sys, urllib.request
context = json.loads(pathlib.Path(os.environ["PROBE_CONTEXT"]).read_text())
def fetch(url, target, *args, **kwargs):
    with pathlib.Path(context["calls"]).open("a") as log:
        log.write(json.dumps(["synthetic-urlretrieve", url]) + "\n")
    if url not in context["urls"]:
        raise SystemExit("unmocked HTTP request: " + url)
    pathlib.Path(target).write_bytes(base64.b64decode(context["urls"][url]))
    return target, None
urllib.request.urlretrieve = fetch
def refuse(*args, **kwargs):
    raise SystemExit("network unavailable in hermetic delegation test")
urllib.request.urlopen = refuse
argv = sys.argv[1:]
if argv[0] == "-c":
    sys.argv = ["-c", *argv[2:]]
    exec(compile(argv[1], "<synthetic-python>", "exec"), {"__name__": "__main__"})
elif argv[0] == "-":
    sys.argv = ["-", *argv[1:]]
    exec(compile(sys.stdin.read(), "<committed-heredoc>", "exec"), {"__name__": "__main__"})
else:
    sys.argv = argv
    runpy.run_path(argv[0], run_name="__main__")
"""
    python = tools / "python3"
    python.write_text(python_wrapper, encoding="utf-8", newline="\n")
    python.chmod(0o755)
    assert static_sha and refuse is not None or True
    del variant


class DelegationWorkspace:
    """A fake repository checkout plus fake tool PATH for the default route."""

    def __init__(self, name: str, refuse: str | None = None, variant: str | None = None) -> None:
        import io
        import tarfile

        self.home = Path(tempfile.mkdtemp(prefix=f"opi-eval-delegate-{name}-"))
        lock = json.loads((REPO_ROOT / STATIC_LOCK).read_text(encoding="utf-8"))
        for pin in lock["authority"]["producers"]:
            target = self.home / pin["path"]
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((REPO_ROOT / pin["path"]).read_bytes())
        workflow = self.home / lock["authority"]["workflow"]["path"]
        workflow.parent.mkdir(parents=True, exist_ok=True)
        workflow.write_bytes((REPO_ROOT / lock["authority"]["workflow"]["path"]).read_bytes())
        for support in (
            "crates/opi-eval/profiles/benchmarks/terminal-bench-2.1.toml",
            "crates/opi-eval/scripts/scripted-provider.py",
        ):
            target = self.home / support
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((REPO_ROOT / support).read_bytes())
        for family in ("harbor", "pier"):
            source = REPO_ROOT / "crates/opi-eval/integrations" / family
            target = self.home / "crates/opi-eval/integrations" / family
            target.mkdir(parents=True, exist_ok=True)
            for module in sorted(source.glob("*.py")):
                (target / module.name).write_bytes(module.read_bytes())

        import hashlib as _hashlib

        def digest(body: bytes) -> str:
            return _hashlib.sha256(body).hexdigest()

        lock["subjects"][1]["commit"] = DELEGATION_COMMIT
        lock["subjects"][1]["tasks_tree"] = "b" * 40
        lock["subjects"][1]["task"] = {
            "id": "fixture",
            "tree": "c" * 40,
            "files": [
                {
                    "path": "task.toml",
                    "git_blob": "d" * 40,
                    "sha256": digest(DELEGATION_TASK),
                    "size": len(DELEGATION_TASK),
                }
            ],
        }
        urls: dict[str, bytes] = {}
        closure = lock["closures"][0]
        for pin in closure["indexes"]:
            body = b"synthetic archive index\n"
            pin["sha256"] = digest(body)
            part = "archive/debian-security" if pin["archive"] == "debian-security" else "archive/debian"
            urls[f"https://snapshot.debian.org/{part}/{closure['apt_epoch']}/{pin['path']}"] = body
        for pin in closure["packages"]:
            body = b"synthetic package archive\n"
            pin["sha256"] = digest(body)
            urls[f"https://snapshot.debian.org/archive/debian/{closure['apt_epoch']}/{pin['path']}"] = body
        for pin in [closure["uv"]["installer"], closure["uv"]["archive"], *closure["wheels"]]:
            body = b"synthetic pinned closure bytes\n"
            pin["sha256"] = digest(body)
            urls[pin["url"]] = body
        image_manifest = (
            json.dumps(
                {
                    "config": {"digest": "sha256:" + "d" * 64},
                    "layers": [{"digest": "sha256:" + "e" * 64}],
                }
            )
            + "\n"
        ).encode()
        lock["images"][0].update(
            manifest="sha256:" + digest(image_manifest),
            config="sha256:" + "d" * 64,
            layers=["sha256:" + "e" * 64],
        )
        lock_path = self.home / STATIC_LOCK
        lock_path.parent.mkdir(parents=True, exist_ok=True)
        lock_path.write_text(json.dumps(lock), encoding="utf-8")

        _delegation_tools(self.home, "", refuse, variant)
        self.tools = self.home / "bin"
        context = {
            "commit": DELEGATION_COMMIT,
            "calls": str(self.home / "calls.jsonl"),
            "refuse": refuse,
            "variant": variant,
            "companion": str(self.tools / "opi-eval"),
            "static_sha": digest(lock_path.read_bytes()),
            "task": __import__("base64").b64encode(DELEGATION_TASK).decode(),
            "urls": {
                url: __import__("base64").b64encode(body).decode() for url, body in urls.items()
            },
            "image_manifest": __import__("base64").b64encode(image_manifest).decode(),
        }
        (self.home / "context.json").write_text(json.dumps(context))
        self.context = context

    def run_default(self) -> subprocess.CompletedProcess[str]:
        out = self.home / "out"
        command = [
            "bash",
            str(self.home / MATERIALIZER),
            "--candidate-commit",
            DELEGATION_COMMIT,
            "--static-lock",
            str(self.home / STATIC_LOCK),
            "--output-dir",
            str(out),
        ]
        env = {
            "PATH": str(self.tools) + ":/usr/bin:/bin",
            "HOME": str(self.home),
            "PROBE_CONTEXT": str(self.home / "context.json"),
            "PYTHONDONTWRITEBYTECODE": "1",
            "GITHUB_WORKFLOW_REF": "OdradekAI/opi/.github/workflows/opi-eval-external-lock-materialization.yml@refs/heads/probe",
            "GITHUB_WORKFLOW_SHA": "f" * 40,
            "GITHUB_RUN_ID": "12345",
            "GITHUB_RUN_ATTEMPT": "1",
        }
        result = subprocess.run(
            command, cwd=self.home, env=env, capture_output=True, text=True, timeout=120
        )
        (self.home / "stdout.log").write_text(result.stdout)
        (self.home / "stderr.log").write_text(result.stderr)
        return result

    def calls(self) -> list[list[str]]:
        return [json.loads(line) for line in Path(self.context["calls"]).read_text().splitlines()]


class DefaultRouteDelegation(unittest.TestCase):
    def test_default_route_delegates_and_publishes_receipt(self) -> None:
        ws = DelegationWorkspace("delegation")
        result = ws.run_default()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = ws.calls()
        self.assertTrue(
            any(call[:2] == ["opi-eval", "prepare"] for call in calls),
            "default materialization never called shared opi-eval prepare",
        )
        self.assertTrue(
            any(call[:2] == ["opi-eval", "conformance"] for call in calls),
            "default materialization never called shared opi-eval conformance",
        )
        receipt_path = ws.home / "out" / "receipt.json"
        self.assertTrue(receipt_path.is_file(), "no receipt after successful shared delegation")
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        self.assertEqual(receipt["schema"], "opi-eval-materialization-receipt/2")
        shared = receipt["shared"]
        self.assertEqual(shared["conformance"]["met"], True)
        self.assertEqual(shared["conformance"]["agent_score"], False)
        self.assertEqual(shared["prepare"]["material"]["schema"], "opi-eval-native-material/7")
        for member in (
            "shared/companion",
            "shared/diagnostic.toml",
            "shared/prepare-report.json",
            "shared/material.json",
            "shared/conformance-report.json",
            "oracle/raw-result.json",
            "oracle/ctrf-report.json",
            "oracle/network-inspection.txt",
        ):
            self.assertTrue((ws.home / "out" / member).is_file(), f"missing {member}")

    def test_shared_prepare_refusal_prevents_receipt(self) -> None:
        ws = DelegationWorkspace("prepare-refusal", refuse="prepare")
        result = ws.run_default()
        self.assertNotEqual(result.returncode, 0, "shared prepare refusal was never observed")
        self.assertFalse((ws.home / "out" / "receipt.json").exists())

    def test_shared_conformance_refusal_prevents_receipt(self) -> None:
        ws = DelegationWorkspace("conformance-refusal", refuse="conformance")
        result = ws.run_default()
        self.assertNotEqual(
            result.returncode, 0, "shared conformance refusal was never observed by default producer"
        )
        self.assertFalse(
            (ws.home / "out" / "receipt.json").exists(),
            "producer published a receipt after shared CLI refusal",
        )

    def test_wrong_task_binding_prevents_receipt(self) -> None:
        ws = DelegationWorkspace("wrong-task", variant="wrong-task")
        result = ws.run_default()
        self.assertNotEqual(result.returncode, 0, "unbound conformance task was accepted")
        self.assertFalse((ws.home / "out" / "receipt.json").exists())

    def test_missing_raw_artifact_prevents_receipt(self) -> None:
        ws = DelegationWorkspace("missing-raw", variant="missing-raw")
        result = ws.run_default()
        self.assertNotEqual(result.returncode, 0, "missing raw verifier artifact was accepted")
        self.assertFalse((ws.home / "out" / "receipt.json").exists())


if __name__ == "__main__":
    unittest.main()
