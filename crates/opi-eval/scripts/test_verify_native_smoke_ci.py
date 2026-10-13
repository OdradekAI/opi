#!/usr/bin/env python3
"""Tests for the opi-eval native-smoke CI contract verifier.

The verifier proves the committed producer contract statically: the manual
workflow, the native-smoke producer, the agent build/locator script, and
the checked-in scripted provider. Tests exercise the real repository files
(the positive contract) plus mutated copies and standalone negative
fixtures for every fail-closed family: dispatch-only triggering, immutable
action pins, runner/timeout/concurrency, workflow-to-script binding,
workflow-byte hashing from the workflow SHA, locked agent builds with
compiler-artifact selection, provider isolation (canonical Python, no-site
mode, one endpoint, one internal network, closed environment), positive
and negative reachability probes, the canary-oracle preflight, no mutable
external identities, no ambient credentials, and seal-before-upload.

This is the smoke-addendum gate for the native-smoke producer contract
(``python crates/opi-eval/scripts/test_verify_native_smoke_ci.py``).
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
VERIFIER = Path(__file__).with_name("verify-native-smoke-ci.py")
WORKFLOW = REPO_ROOT / ".github/workflows/opi-eval-native-smoke.yml"
PRODUCER = REPO_ROOT / "crates/opi-eval/scripts/native-smoke.sh"
BUILDER = REPO_ROOT / "crates/opi-eval/scripts/build-agent-artifacts.sh"
PROVIDER = REPO_ROOT / "crates/opi-eval/scripts/scripted-provider.py"
FIXTURES = Path(__file__).parent / "fixtures/native-smoke-ci"


class Workspace:
    """A temporary workspace seeded with copies of the real producer files."""

    def __init__(self, workflow_text: str | None = None,
                 producer_text: str | None = None,
                 builder_text: str | None = None) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="native-smoke-ci-"))
        self.workflow_dir = self.root / ".github/workflows"
        self.workflow_dir.mkdir(parents=True)
        self.scripts = self.root / "scripts"
        self.scripts.mkdir()
        self.workflow = self.workflow_dir / "opi-eval-native-smoke.yml"
        self.workflow.write_text(
            workflow_text if workflow_text is not None
            else WORKFLOW.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.producer = self.scripts / "native-smoke.sh"
        self.producer.write_text(
            producer_text if producer_text is not None
            else PRODUCER.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.builder = self.scripts / "build-agent-artifacts.sh"
        self.builder.write_text(
            builder_text if builder_text is not None
            else BUILDER.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.provider = self.scripts / "scripted-provider.py"
        shutil.copyfile(PROVIDER, self.provider)

    def verify(self, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(VERIFIER),
             "--workflow", str(self.workflow),
             "--script", str(self.producer),
             "--build-script", str(self.builder),
             "--provider", str(self.provider), *extra],
            capture_output=True, text=True, cwd=REPO_ROOT)

    def cleanup(self) -> None:
        shutil.rmtree(self.root, ignore_errors=True)


class NativeSmokeCiVerifier(unittest.TestCase):
    def setUp(self) -> None:
        self._workspaces: list[Workspace] = []

    def tearDown(self) -> None:
        for ws in self._workspaces:
            ws.cleanup()

    def ws(self, **overrides: str) -> Workspace:
        ws = Workspace(**overrides)  # type: ignore[arg-type]
        self._workspaces.append(ws)
        return ws

    def assert_rejects(self, ws: Workspace, needle: str) -> None:
        completed = ws.verify()
        self.assertNotEqual(
            completed.returncode, 0,
            f"expected rejection, stdout: {completed.stdout}")
        self.assertIn(needle, completed.stdout)

    # -- argument contract ---------------------------------------------------
    def test_missing_workflow_argument_rejects(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(VERIFIER),
             "--script", str(PRODUCER),
             "--build-script", str(BUILDER),
             "--provider", str(PROVIDER)],
            capture_output=True, text=True, cwd=REPO_ROOT)
        self.assertNotEqual(completed.returncode, 0)

    def test_missing_script_argument_rejects(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(VERIFIER),
             "--workflow", str(WORKFLOW),
             "--build-script", str(BUILDER),
             "--provider", str(PROVIDER)],
            capture_output=True, text=True, cwd=REPO_ROOT)
        self.assertNotEqual(completed.returncode, 0)

    # -- the real committed contract accepts ---------------------------------
    def test_real_repository_files_accept(self) -> None:
        completed = subprocess.run(
            [sys.executable, str(VERIFIER),
             "--workflow", str(WORKFLOW),
             "--script", str(PRODUCER),
             "--build-script", str(BUILDER),
             "--provider", str(PROVIDER)],
            capture_output=True, text=True, cwd=REPO_ROOT)
        self.assertEqual(
            completed.returncode, 0,
            f"real producer contract rejected: {completed.stdout}")

    # -- workflow trigger and identity families ------------------------------
    def test_schedule_trigger_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "on:\n  workflow_dispatch:",
                "on:\n  schedule:\n    - cron: '0 3 * * *'\n  workflow_dispatch:")),
            "trigger")

    def test_push_trigger_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "on:\n  workflow_dispatch:",
                "on:\n  push:\n    branches: [main]\n  workflow_dispatch:")),
            "trigger")

    def test_floating_action_tag_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683",
                "actions/checkout@v4")),
            "action")

    def test_unrecorded_action_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "      - name: Record the host identity",
                "      - name: Sneak an unrecorded action\n"
                "        uses: someone/else@1234567890abcdef1234567890abcdef12345678\n"
                "      - name: Record the host identity")),
            "action")

    def test_wrong_runner_label_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "runs-on: ubuntu-24.04", "runs-on: ubuntu-latest")),
            "runner")

    def test_missing_timeout_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "    timeout-minutes: 360\n", "")),
            "timeout")

    def test_missing_candidate_binding_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "          CANDIDATE: ${{ inputs.candidate_sha }}\n", "")),
            "candidate")

    def test_missing_canary_preflight_step_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "          bash crates/opi-eval/scripts/native-smoke.sh preflight-canaries",
                "          bash crates/opi-eval/scripts/native-smoke.sh host-identity")),
            "canary-preflight")

    def test_upload_before_seal_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "path: ${{ runner.temp }}/opi-eval-native/08-seal/sealed-artifact.tar",
                "path: ${{ runner.temp }}/opi-eval-native")),
            "upload")

    # -- producer script contract families ------------------------------------
    def test_workflow_bytes_not_read_from_workflow_sha_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'git -C "$REPO_ROOT" show "${workflow_sha}:${workflow_path}"',
                'git -C "$REPO_ROOT" show "HEAD:${workflow_path}"')),
            "workflow-sha")

    def test_provider_without_no_site_python_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                '"$(readlink -f "$(command -v python3)")" -I -S "$provider" \\',
                '"$(readlink -f "$(command -v python3)")" "$provider" \\')),
            "no-site")

    def test_provider_with_ambient_environment_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'env -i PATH="/usr/bin:/bin" HOME="$out" \\',
                'PATH="/usr/bin:/bin" \\')),
            "environment")

    def test_second_listener_endpoint_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                "for port in (48127, 48128, 48129, 48130):",
                "for port in (48127, 48128, 48129, 48130, 48131):\n"
                "    pass\n"
                "for port in (48127, 48128, 48129, 48130):")),
            "endpoint")

    def test_external_network_egress_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'docker network create --internal "$network" >/dev/null',
                'docker network create "$network" >/dev/null')),
            "network")

    def test_missing_negative_reachability_probe_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'die "provider-probe: the endpoint answers on a non-loopback interface"',
                'die "provider-probe: unreachable"')),
            "negative-probe")

    def test_mutable_external_ref_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'subprocess.run(["git", "clone", "--quiet", "--filter=blob:none",',
                'subprocess.run(["git", "clone", "--quiet", "--branch", "main", "--filter=blob:none",')),
            "mutable")

    def test_unlocked_trial_build_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                "build_json=$(cargo build --locked --release -p opi-eval \\",
                "build_json=$(cargo build --release -p opi-eval \\")),
            "locked")

    def test_missing_canary_oracle_material_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                'if not pinned["path"].startswith(("solution/", "tests/")):',
                'if pinned["path"].startswith(("solution/", "tests/")):')),
            "oracle")

    def test_missing_materialize_stage_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "crates/opi-eval/scripts/native-smoke.sh materialize-configs",
                "crates/opi-eval/scripts/native-smoke.sh verify-dispatch")),
            "materialize")

    def test_missing_oracle_preflight_stage_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "crates/opi-eval/scripts/native-smoke.sh oracle-preflight",
                "crates/opi-eval/scripts/native-smoke.sh verify-dispatch")),
            "stage")

    def test_ambient_credential_usage_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                '"ambient_credentials": "none",',
                '"ambient_credentials": os.environ.get("OPENAI_API_KEY", "none"),')),
            "credential")


    # -- shared local preparation contract ------------------------------------
    def test_materialize_without_shared_prepare_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                '"prepare", "--config"', '"validate", "--config"')),
            "materialize")

    def test_materialize_with_duplicate_task_transform_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                "def sha(path):",
                "def materialize_task(benchmark):\n"
                "    raise SystemExit('duplicate policy')\n\n\n"
                "def sha(path):")),
            "materialize")

    def test_workflow_with_synthesized_material_path_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                '--material-root "$OPI_EVAL_NATIVE_STAGE/06-material"',
                '--material "$OPI_EVAL_NATIVE_STAGE/06-material/material.json"')),
            "materialize")

    def test_materialize_configs_delegates_to_shared_prepare(self) -> None:
        """The public Bash stage routes materialization through the shared
        local prepare path: an accepted offline fixture preparation
        publishes per-family prepared material with identical
        task/runner identities, and a shared prepare refusal stops the
        stage before any material is published."""
        import hashlib
        for refuse in (False, True):
            with tempfile.TemporaryDirectory(prefix="native-smoke-bash-") as temporary:
                root = Path(temporary)
                bin_dir = root / "bin"
                external = root / "external"
                agents = root / "agents"
                provider = root / "provider"
                out = root / "out"
                calls = root / "calls.jsonl"
                bin_dir.mkdir()
                lock = json.loads(
                    (REPO_ROOT / "crates/opi-eval/external-locks/static"
                     / "linux-x86_64.json").read_text(encoding="utf-8"))
                clone_index = {"terminal-bench-2.1": "terminal-bench-2-1",
                               "terminal-bench-3.0": "terminal-bench",
                               "deepswe-v1.1": "deep-swe"}
                for family, clone in clone_index.items():
                    pin = next(s for s in lock["subjects"] if s["id"] == family)
                    task = external / clone / "tasks" / pin["task"]["id"]
                    source = REPO_ROOT / "crates/opi-eval/tests/fixtures/benchmarks" / family / "task-package"
                    shutil.copytree(source, task)
                    (task / "task.toml").write_text(
                        "[task]\nid = " + json.dumps(pin["task"]["id"])
                        + "\n[environment]\n[verifier]\ntimeout_sec = 1\n",
                        encoding="utf-8")
                    for dockerfile in task.rglob("Dockerfile"):
                        dockerfile.write_text("FROM scratch\n", encoding="utf-8")
                for runner in ("harbor", "pier"):
                    runner_dir = external / runner
                    runner_dir.mkdir()
                    (runner_dir / "uv.lock").write_text("version = 1\n", encoding="utf-8")
                opi = root / "opi"
                node = root / "node"
                package = root / "pi-package"
                bundle = package / "dist/bundle/cli.js"
                opi.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                opi.chmod(0o755)
                node.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
                node.chmod(0o755)
                bundle.parent.mkdir(parents=True)
                bundle.write_text("fixture", encoding="utf-8")
                for name in ("docs", "examples"):
                    (package / name).mkdir()
                for name in ("package.json", "README.md", "CHANGELOG.md"):
                    (package / name).write_text("fixture", encoding="utf-8")

                def sha(path):
                    if isinstance(path, bytes):
                        return hashlib.sha256(path).hexdigest()
                    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

                agents.mkdir()
                provider.mkdir()
                (agents / "opi-identity.json").write_text(json.dumps({
                    "canonical_executable": str(opi),
                    "executable_sha256": sha(opi)}), encoding="utf-8")
                (agents / "pi-identity.json").write_text(json.dumps({
                    "node_executable": str(node), "node_sha256": sha(node),
                    "bundle_path": str(bundle), "bundle_sha256": sha(bundle)}),
                    encoding="utf-8")
                (provider / "endpoint.txt").write_text("127.0.0.1:48127", encoding="utf-8")
                lock_file = (REPO_ROOT / "crates/opi-eval/external-locks/static"
                             / "linux-x86_64.json")
                template = {
                    "schema": "opi-eval-native-material/7", "origin": "local",
                    "static_lock": {"path": str(lock_file), "sha256": sha(lock_file)},
                    "capture_helper": {
                        "executable": {"path": str(agents / "capture-helper"),
                                       "sha256": sha(b"capture-helper\n")},
                        "target": "x86_64-unknown-linux-musl",
                        "protocol": "opi-eval-native-capture-helper/1",
                        "cargo_lock_sha256": "0" * 64,
                    },
                    "provider": {
                        "script": {"path": "unused",
                                   "sha256": sha(REPO_ROOT / "crates/opi-eval/scripts/scripted-provider.py")},
                        "endpoint": "http://127.0.0.1:48127", "request_log": "unused"},
                    "agents": {
                        "opi": {"executable": {"path": str(opi), "sha256": sha(opi)},
                                "integrations": {}},
                        "pi": {"executable": {"path": str(package / "pi"), "sha256": "fixture"},
                               "integrations": {}},
                    },
                    "benchmark": {
                        "corpus": {"root": "unused", "source_commit": "a" * 40,
                                   "tasks_tree": "b" * 40},
                        "tasks": {"fixture-task": {
                            "native_task_name": "fixture-task",
                            "profile": {"path": "unused", "sha256": "c" * 64},
                            "task_package": "unused",
                            "task_package_manifest_sha256": "d" * 64}},
                        "verifier_executable": {"path": "unused", "sha256": "e" * 64},
                        "verifier_env": {},
                        "runner_project": {
                            "path": "/fixture/runners",
                            "source_manifest_sha256": "f" * 64,
                            "interpreter": {"path": "/fixture/runners/.venv/bin/python",
                                            "sha256": "1" * 64},
                            "entrypoint": {"path": "/fixture/runners/.venv/bin/runner",
                                           "sha256": "2" * 64}},
                        "oracle": {"path": "unused", "sha256": "e" * 64},
                        "oracle_env": {},
                    },
                }
                template_path = root / "identity-template.json"
                template_path.write_text(json.dumps(template, indent=2, sort_keys=True),
                                         encoding="utf-8")
                stub_body = (
                    "#!/usr/bin/env python3\n"
                    "import json, pathlib, sys\n"
                    "template = pathlib.Path(__TEMPLATE__)\n"
                    "calls = pathlib.Path(__CALLS__)\n"
                    "refuse = __REFUSE__\n"
                    "argv = sys.argv[1:]\n"
                    "with calls.open('a') as log:\n"
                    "    log.write(json.dumps(argv) + chr(10))\n"
                    "if argv[0] == 'prepare':\n"
                    "    if refuse:\n"
                    "        raise SystemExit('shared-preparation-refused')\n"
                    "    config = pathlib.Path(argv[argv.index('--config') + 1])\n"
                    "    out = pathlib.Path(argv[argv.index('--out') + 1])\n"
                    "    out.mkdir(parents=True, exist_ok=True)\n"
                    "    identity = json.loads(template.read_text())\n"
                    "    identity['benchmarks'] = {config.stem: identity.pop('benchmark')}\n"
                    "    (out / 'material.json').write_text(json.dumps(identity, indent=2, sort_keys=True))\n"
                    "elif argv[0] == 'validate':\n"
                    "    print('native_integrity=' + 'a' * 64)\n"
                    "else:\n"
                    "    raise SystemExit('unexpected CLI call: ' + str(argv))\n"
                ).replace("__TEMPLATE__", repr(str(template_path))
                ).replace("__CALLS__", repr(str(calls))
                ).replace("__REFUSE__", str(int(refuse)))
                companion = bin_dir / "opi-eval"
                companion.write_text(stub_body, encoding="utf-8")
                companion.chmod(0o755)
                cargo = bin_dir / "cargo"
                cargo.write_text(
                    "#!/usr/bin/env python3\n"
                    "import os, pathlib, sys\n"
                    "if sys.argv[1] == 'test':\n"
                    "    root = pathlib.Path(os.environ['OPI_EVAL_OUTPUT_CONTROL_OUT'])\n"
                    "    for product in ('opi', 'pi'):\n"
                    "        (root / product).mkdir(parents=True)\n"
                    "        (root / product / 'output-control.json').write_text('{}')\n"
                    "else:\n"
                    "    import json as _json\n"
                    "    print(_json.dumps({'reason': 'compiler-artifact',\n"
                    "                       'target': {'name': 'opi-eval'},\n"
                    "                       'executable': " + repr(str(companion)) + "}))\n",
                    encoding="utf-8")
                cargo.chmod(0o755)
                for name, body in (
                        ("git", "#!/bin/sh\nprintf '%s\\n' " + "a" * 40 + "\n"),
                        ("uv", "#!/bin/sh\nexit 0\n")):
                    stub = bin_dir / name
                    stub.write_text(body, encoding="utf-8")
                    stub.chmod(0o755)
                env = {"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(root)}
                result = subprocess.run(
                    ["bash", str(PRODUCER), "materialize-configs",
                     "--external-root", str(external),
                     "--agents-out", str(agents),
                     "--provider-out", str(provider),
                     "--out", str(out)],
                    env=env, text=True, capture_output=True, cwd=str(REPO_ROOT))
                if refuse:
                    self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                    self.assertIn("shared-preparation-refused", result.stderr)
                    self.assertFalse((out / "material-routing.json").exists(),
                                     "CI published material after a shared prepare refusal")
                    continue
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                invocations = [json.loads(line) for line in calls.read_text().splitlines()]
                self.assertTrue(any(argv[0] == "prepare" for argv in invocations),
                                "materialization did not delegate to shared preparation")
                routing = json.loads((out / "material-routing.json").read_text(encoding="utf-8"))
                self.assertEqual(routing["schema"], "opi-eval-native-material-routing/1")
                self.assertEqual(sorted(routing["adapters"]),
                                 ["deepswe-v1.1", "terminal-bench-2.1", "terminal-bench-3.0"])
                runner_projects = {}
                for adapter, entry in routing["adapters"].items():
                    material_path = Path(entry["path"])
                    self.assertTrue(material_path.is_file())
                    self.assertEqual(sha(material_path), entry["sha256"])
                    material = json.loads(material_path.read_text(encoding="utf-8"))
                    self.assertEqual(material["schema"], "opi-eval-native-material/7")
                    self.assertEqual(material["origin"], "local")
                    runner_projects[adapter] = material["benchmarks"][adapter]["runner_project"]
                # One shared preparation identity: identical runner source
                # admission across every family.
                first = runner_projects["terminal-bench-2.1"]
                for adapter in ("terminal-bench-3.0", "deepswe-v1.1"):
                    self.assertEqual(runner_projects[adapter]["source_manifest_sha256"],
                                     first["source_manifest_sha256"])
                receipt = json.loads((out / "materialize-receipt.json").read_text(encoding="utf-8"))
                self.assertEqual(receipt["preparation"], "shared-local-prepare")

    # -- conformance receipt count -------------------------------------------
    def test_conformance_receipt_count_derives_from_the_case_list(self) -> None:
        text = PRODUCER.read_text(encoding="utf-8")
        # The declared case list is the single authority for the count.
        case_list = text.split("for case_spec in", 1)[1].split("; do", 1)[0]
        declared = case_list.count('"agent ') + case_list.count('"benchmark ')
        self.assertGreater(declared, 0)
        # The receipt serializes the loop counter, never an independent
        # hand-maintained literal: the counter is initialized before the
        # loop, incremented only after a successful case, and passed into
        # the receipt serialization.
        self.assertIn("cases_run=0", text)
        self.assertIn("cases_run=$((cases_run + 1))", text)
        self.assertIn(
            '{"cases_run": int(sys.argv[1]), "mode": "native-material"}',
            text,
        )
        self.assertIn('"$cases_run")', text)
        # No independent count literal survives anywhere in the producer.
        self.assertNotIn('"cases_run": 1', text)
        self.assertNotIn('"cases_run": 2', text)

    # -- build/locator script contract families -------------------------------
    def test_build_agents_forwards_static_lock_and_records_exact_argv(self) -> None:
        with tempfile.TemporaryDirectory(prefix="native-smoke-caller-") as temporary:
            root = Path(temporary)
            scripts = root / "crates/opi-eval/scripts"
            scripts.mkdir(parents=True)
            producer = scripts / PRODUCER.name
            shutil.copyfile(PRODUCER, producer)
            lock = root / "crates/opi-eval/external-locks/static/linux-x86_64.json"
            lock.parent.mkdir(parents=True)
            lock.write_text('{"subjects": []}\n')
            subprocess.run(["git", "init", "--quiet", str(root)], check=True)
            subprocess.run(["git", "-C", str(root), "-c", "user.name=Caller Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "--allow-empty", "-m", "test: caller fixture"], check=True)
            builder = scripts / "record-builder.sh"
            builder.write_text('''#!/usr/bin/env bash
set -euo pipefail
python3 - "$@" <<'PY'
import json, pathlib, sys
args = sys.argv[1:]
out = pathlib.Path(args[args.index("--out") + 1])
out.mkdir(parents=True)
(out / "argv.json").write_text(json.dumps(args))
(out / "opi-identity.json").write_text(json.dumps({"canonical_executable": "/fixture/opi", "checkout_commit": args[args.index("--opi-commit") + 1]}))
(out / "pi-identity.json").write_text(json.dumps({"node_executable": "/fixture/node", "bundle_path": "/fixture/pi.js"}))
PY
''', newline="\n")
            out = root / "output"
            pi_source = root / "pi-source"
            result = subprocess.run(["bash", str(producer), "build-agents", "--build-script", str(builder), "--pi-source", str(pi_source), "--out", str(out)], cwd=root, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            args = json.loads((out / "argv.json").read_text())
            self.assertEqual(args[args.index("--static-lock") + 1], str(lock))
            receipt = json.loads((out / "receipt.json").read_text())
            self.assertEqual(receipt["build_script_invocation"], ["bash", str(builder), *args])

    def test_build_without_locked_release_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                'cargo build --locked --release --target "$OPI_TARGET" \\',
                'cargo build --target "$OPI_TARGET" \\')),
            "locked")

    def test_build_assuming_target_path_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                '"compiler-artifact"', '"never"')),
            "compiler-artifact")

    def test_build_missing_executable_identity_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                '    "file": run(["file", "-b", executable]).strip(),',
                '')),
            "identity")

    def test_pi_build_without_locked_install_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                "    npm ci --ignore-scripts",
                "    npm install")),
            "npm-ci")

    def test_pi_build_missing_bundle_check_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                "  test -f packages/coding-agent/dist/bundle/cli.js",
                "  true").replace(
                "  printf 'test -f packages/coding-agent/dist/bundle/cli.js\\n'",
                "  printf 'true\\n'")),
            "bundle")

    # -- provider boundary -----------------------------------------------------
    def test_provider_with_third_party_dependency_rejects(self) -> None:
        ws = self.ws()
        text = ws.provider.read_text(encoding="utf-8")
        ws.provider.write_text(
            "import requests\n" + text, encoding="utf-8", newline="\n")
        self.assert_rejects(ws, "provider")

    def test_provider_missing_identity_schema_rejects(self) -> None:
        ws = self.ws()
        text = ws.provider.read_text(encoding="utf-8")
        ws.provider.write_text(
            text.replace('SCHEMA = "opi-eval-scripted-provider/1"', 'SCHEMA = "x"'),
            encoding="utf-8", newline="\n")
        self.assert_rejects(ws, "provider")

    # -- standalone negative fixtures ------------------------------------------
    def test_fixture_floating_action_workflow_rejects(self) -> None:
        ws = self.ws()
        shutil.copyfile(FIXTURES / "workflow-floating-action.yml", ws.workflow)
        self.assert_rejects(ws, "action")

    def test_fixture_second_listener_producer_rejects(self) -> None:
        ws = self.ws()
        shutil.copyfile(FIXTURES / "producer-second-listener.sh", ws.producer)
        self.assert_rejects(ws, "endpoint")


if __name__ == "__main__":
    unittest.main()
