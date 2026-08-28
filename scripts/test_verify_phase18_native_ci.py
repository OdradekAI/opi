#!/usr/bin/env python3
"""Tests for the Phase 18 native-smoke CI contract verifier (task 18.14).

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

This is the smoke-addendum gate for task 18.14
(``python scripts/test_verify_phase18_native_ci.py``).
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
VERIFIER = Path(__file__).with_name("verify-phase18-native-ci.py")
WORKFLOW = REPO_ROOT / ".github/workflows/phase18-native-smoke.yml"
PRODUCER = REPO_ROOT / "scripts/phase18-native-smoke.sh"
BUILDER = REPO_ROOT / "scripts/phase18-build-agent-artifacts.sh"
PROVIDER = REPO_ROOT / "scripts/phase18-scripted-provider.py"
FIXTURES = Path(__file__).parent / "fixtures/phase18-native-ci"


class Workspace:
    """A temporary workspace seeded with copies of the real producer files."""

    def __init__(self, workflow_text: str | None = None,
                 producer_text: str | None = None,
                 builder_text: str | None = None) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="phase18-native-ci-"))
        self.workflow_dir = self.root / ".github/workflows"
        self.workflow_dir.mkdir(parents=True)
        self.scripts = self.root / "scripts"
        self.scripts.mkdir()
        self.workflow = self.workflow_dir / "phase18-native-smoke.yml"
        self.workflow.write_text(
            workflow_text if workflow_text is not None
            else WORKFLOW.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.producer = self.scripts / "phase18-native-smoke.sh"
        self.producer.write_text(
            producer_text if producer_text is not None
            else PRODUCER.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.builder = self.scripts / "phase18-build-agent-artifacts.sh"
        self.builder.write_text(
            builder_text if builder_text is not None
            else BUILDER.read_text(encoding="utf-8"),
            encoding="utf-8", newline="\n")
        self.provider = self.scripts / "phase18-scripted-provider.py"
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


class Phase18NativeCiVerifier(unittest.TestCase):
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
                "          bash scripts/phase18-native-smoke.sh preflight-canaries",
                "          bash scripts/phase18-native-smoke.sh host-identity")),
            "canary-preflight")

    def test_upload_before_seal_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "path: ${{ runner.temp }}/phase18-native/08-seal/sealed-artifact.tar",
                "path: ${{ runner.temp }}/phase18-native")),
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
                "scripts/phase18-native-smoke.sh materialize-configs",
                "scripts/phase18-native-smoke.sh verify-dispatch")),
            "materialize")

    def test_missing_oracle_preflight_stage_rejects(self) -> None:
        self.assert_rejects(
            self.ws(workflow_text=WORKFLOW.read_text(encoding="utf-8").replace(
                "scripts/phase18-native-smoke.sh oracle-preflight",
                "scripts/phase18-native-smoke.sh verify-dispatch")),
            "stage")

    def test_ambient_credential_usage_rejects(self) -> None:
        self.assert_rejects(
            self.ws(producer_text=PRODUCER.read_text(encoding="utf-8").replace(
                '"ambient_credentials": "none",',
                '"ambient_credentials": os.environ.get("OPENAI_API_KEY", "none"),')),
            "credential")

    # -- build/locator script contract families -------------------------------
    def test_build_without_locked_release_rejects(self) -> None:
        self.assert_rejects(
            self.ws(builder_text=BUILDER.read_text(encoding="utf-8").replace(
                "cargo build --locked --release \\",
                "cargo build \\")),
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
            text.replace('SCHEMA = "phase18-scripted-provider/1"', 'SCHEMA = "x"'),
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
