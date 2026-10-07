#!/usr/bin/env python3
"""Shared offline conformance for the actual Harbor and Pier integrations.

Only the external runner modules and async task environment are fakes. The
integration classes load real temporary handoffs, validate and upload runtime
files, launch through the environment API, and retain evidence before cleanup.
Run: python crates/opi-eval/scripts/test_native_integrations.py
"""

from __future__ import annotations

import asyncio
import importlib.util
import hashlib
import json
import logging
import os
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import types
import unittest
from pathlib import Path, PurePosixPath
from unittest.mock import patch


# Importing the integration must not add bytecode to its pinned module tree.
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
AGENT_SHA = "d4f0bc5a29de06b510f9aa428f1eedba926012b591fef7a518e776a7c9bd1824"


def capture_helper_path():
    """Locate the built Companion for the staged capture helper.

    The real filter must run at the real first sink; a shell stand-in
    cannot model the privacy boundary. Resolution order: an explicit
    environment override, then the workspace target directory.
    """
    override = os.environ.get("OPI_EVAL_CAPTURE_HELPER")
    if override:
        return Path(override)
    candidates = [
        Path(__file__).resolve().parents[3] / "target" / "debug" / "opi-eval",
    ]
    cache_root = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "opi" / "cargo-targets"
    if cache_root.is_dir():
        candidates.extend(sorted(cache_root.glob("*/debug/opi-eval"), key=lambda p: p.stat().st_mtime, reverse=True))
    for candidate in candidates:
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    raise RuntimeError(
        "no built opi-eval Companion found for the capture helper; "
        "build the workspace or set OPI_EVAL_CAPTURE_HELPER"
    )


def capture_helper_block():
    helper = capture_helper_path()
    return {
        "path": str(helper),
        "sha256": hashlib.sha256(helper.read_bytes()).hexdigest(),
        "target": "x86_64-unknown-linux-gnu",
        "protocol": "opi-eval-native-capture-helper/1",
    }
# Canonical path\nsha256\n manifest for the four fixed bytestrings below.
PACKAGE_SHA = "c633facd2c08421e9d2e145e855c64fae14eb50f1a4048d52436164363de19d8"
PACKAGE_FILES = {
    "pi": b"agent",
    "package.json": b"{}",
    "theme/dark.json": b"dark",
    "theme/light.json": b"light",
}
STDOUT = b'{"type":"tool_execution_end","toolName":"bash"}\n'
STDERR = b"agent diagnostic\n"
HEADLESS_BUS = "unix:abstract=/opi-eval-agent/opi/no-keyring"


class RunnerBaseAgent:
    def __init__(self, logs_dir, model_name=None, logger=None, **kwargs):
        self.logs_dir = logs_dir
        self.logger = logger or logging.getLogger("native-integration-test")


class NonZeroAgentExitCodeError(RuntimeError):
    pass


class RunnerPierBaseAgent(RunnerBaseAgent):
    def network_allowlist(self):
        return types.SimpleNamespace(domains=[])


class RunnerDockerEnvironment:
    """Pinned exec transport seam, backed by real isolated shell commands."""

    def __init__(self, root):
        self.work = root
        self.trial_paths = types.SimpleNamespace(trial_dir=root)
        self.session_id = root.name
        self._keep_containers = False
        self.commands = []

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None):
        proc = await asyncio.create_subprocess_exec(
            "bash", "-c", command, cwd=cwd or self.work,
            env={"PATH": "/usr/bin:/bin", **(env or {})},
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        return types.SimpleNamespace(return_code=proc.returncode,
                                     stdout=stdout.decode(), stderr=stderr.decode())

    async def _run_docker_compose_command(self, command, **kwargs):
        self.commands.append(command)
        return types.SimpleNamespace(return_code=0, stdout="", stderr="")

    async def prepare_logs_for_host(self):
        pass

    async def stop(self, delete):
        await self._run_docker_compose_command(["down", "--rmi", "all", "--volumes"])


def load_integration(family):
    """Stub only imported runner types; execute the real integration module."""
    modules = {}
    for suffix in ("", ".agents", ".agents.base", ".agents.installed",
                   ".agents.installed.base", ".environments", ".environments.base",
                   ".models", ".models.agent", ".models.agent.context",
                   ".models.agent.network", ".environments.docker",
                   ".environments.docker.docker"):
        name = family + suffix
        modules[name] = types.ModuleType(name)
    modules[f"{family}.agents.base"].BaseAgent = (
        RunnerPierBaseAgent if family == "pier" else RunnerBaseAgent
    )
    modules[f"{family}.agents.installed.base"].NonZeroAgentExitCodeError = NonZeroAgentExitCodeError
    modules[f"{family}.environments.base"].BaseEnvironment = object
    modules[f"{family}.environments.docker.docker"].DockerEnvironment = RunnerDockerEnvironment
    modules[f"{family}.models.agent.context"].AgentContext = object
    modules[f"{family}.models.agent.network"].NetworkAllowlist = types.SimpleNamespace
    path = ROOT / "integrations" / family / f"opi_eval_{family}.py"
    spec = importlib.util.spec_from_file_location(f"test_opi_eval_{family}", path)
    module = importlib.util.module_from_spec(spec)
    with patch.dict(sys.modules, modules):
        spec.loader.exec_module(module)
    return module, modules


INTEGRATIONS = {family: load_integration(family) for family in ("harbor", "pier")}


@unittest.skipUnless(os.name == "posix", "requires Linux process groups and shell")
class NativeEnvironmentTest(unittest.IsolatedAsyncioTestCase):
    async def test_runtime_capture_separates_bounds_and_observes_timeout(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module.OwnedDockerEnvironment(Path(root))
                observed = await environment.exec(
                    "printf 'version\\n'; printf 'diagnostic\\n' >&2; exit 7",
                    timeout_sec=2, _capture_limit=64)
                self.assertEqual(observed.return_code, 7)
                self.assertEqual(observed.stdout, b"version\n")
                self.assertEqual(observed.stderr, b"diagnostic\n")
                overflow = await environment.exec(
                    "printf '123456789abcdef'", timeout_sec=2, _capture_limit=8)
                self.assertEqual(overflow.stdout, b"123456789")
                with self.assertRaises(TimeoutError) as failure:
                    await environment.exec(
                        "printf partial; sleep 1; touch escaped", timeout_sec=0.05,
                        _capture_limit=64)
                self.assertEqual(failure.exception.runtime_capture.stdout, b"partial")
                self.assertEqual(failure.exception.runtime_capture.return_code, 137)
                await asyncio.sleep(1.1)
                self.assertFalse((Path(root) / "escaped").exists())

    async def test_native_exec_preserves_command_umask(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                baseline = await RunnerDockerEnvironment(Path(root)).exec("umask")
                observed = await module.OwnedDockerEnvironment(Path(root)).exec("umask")
                self.assertEqual(observed.stdout, baseline.stdout)

    async def test_native_exec_cancellation_stops_effects_and_observes_exit(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                root = Path(root)
                environment = module.OwnedDockerEnvironment(root)
                launch = asyncio.create_task(environment.exec(
                    "touch started; (sleep 0.6; touch escaped) & wait"))
                async with asyncio.timeout(2):
                    while not (root / "started").exists():
                        await asyncio.sleep(0.01)
                launch.cancel()
                await asyncio.sleep(0.01)
                launch.cancel()
                with self.assertRaises(asyncio.CancelledError):
                    await launch
                await asyncio.sleep(0.7)
                self.assertFalse((root / "escaped").exists())
                rows = [json.loads(row) for row in
                        (root / "opi-eval-commands.jsonl").read_text().splitlines()]
                self.assertEqual(len(rows), 1)
                self.assertEqual(rows[0]["exit_code"], 137)
                self.assertEqual(rows[0]["termination"], "cancelled")

    async def test_native_exec_completion_timeout_and_race(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module.OwnedDockerEnvironment(Path(root))
                for code in (0, 7):
                    result = await environment.exec(f"printf result; exit {code}")
                    self.assertEqual(result.return_code, code)
                    self.assertEqual(result.stdout, "result")
                with self.assertRaises(TimeoutError):
                    await environment.exec("sleep 0.6; touch escaped", timeout_sec=0.05)
                for _ in range(5):
                    launch = asyncio.create_task(environment.exec("sleep 0.01"))
                    await asyncio.sleep(0.01)
                    launch.cancel()
                    try:
                        await launch
                    except asyncio.CancelledError:
                        pass
                await asyncio.sleep(0.7)
                self.assertFalse((Path(root) / "escaped").exists())

    async def test_native_stop_preserves_shared_images_and_surfaces_failure(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module.OwnedDockerEnvironment(Path(root))
                await environment.stop(delete=True)
                self.assertIn(["down", "--volumes"], environment.commands)
                self.assertFalse(any("--rmi" in cmd for cmd in environment.commands))
                async def failed(*args, **kwargs):
                    raise RuntimeError("cleanup unavailable")
                with patch.object(RunnerDockerEnvironment, "_run_docker_compose_command", failed):
                    with self.assertRaisesRegex(RuntimeError, "cleanup unavailable"):
                        await environment.stop(delete=True)

    async def test_native_stop_settles_before_propagating_cancellation(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module.OwnedDockerEnvironment(Path(root))
                settled = asyncio.Event()
                async def slow_stop(*args, **kwargs):
                    await asyncio.sleep(0.1)
                    settled.set()
                with patch.object(RunnerDockerEnvironment, "stop", slow_stop):
                    stop = asyncio.create_task(environment.stop(delete=True))
                    await asyncio.sleep(0.01)
                    stop.cancel()
                    with self.assertRaises(asyncio.CancelledError):
                        await stop
                self.assertTrue(settled.is_set())

    async def test_failed_command_termination_escalates_only_owned_environment(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module.OwnedDockerEnvironment(Path(root))
                original = RunnerDockerEnvironment.exec
                async def broken_stop(instance, command, **kwargs):
                    if kwargs.get("user") == "root":
                        raise RuntimeError("termination transport failed")
                    return await original(instance, command, **kwargs)
                with patch.object(RunnerDockerEnvironment, "exec", broken_stop):
                    launch = asyncio.create_task(environment.exec("sleep 0.2"))
                    await asyncio.sleep(0.02)
                    launch.cancel()
                    with self.assertRaisesRegex(RuntimeError, "termination is uncertain"):
                        await launch
                self.assertEqual(environment.commands, [["stop"]])
                self.assertTrue((Path(root) / "opi-eval-effect-unknown").exists())


class VerifierOnlyEnvironmentTest(unittest.IsolatedAsyncioTestCase):
    async def test_offline_verifier_never_builds_or_pulls(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module._VerifierDockerEnvironment(Path(root))
                environment._env_vars = types.SimpleNamespace(main_image_name="native-cached-image")
                environment.task_env_config = types.SimpleNamespace(docker_image=None)
                environment._use_prebuilt = False
                inspections = []
                async def cached(*args):
                    inspections.append(args)
                    return "sha256:" + "a" * 64
                with patch.object(module, "_regrade_docker", cached):
                    await environment._run_docker_compose_command(["build"])
                    self.assertEqual(environment.commands, [])
                    await environment._run_docker_compose_command(["up", "--detach", "--wait"])
                    self.assertEqual(environment.commands, [["up", "--detach", "--wait", "--no-build", "--pull", "never"]])
                    self.assertEqual(environment.regrade_image["id"], "sha256:" + "a" * 64)
                    with self.assertRaisesRegex(ValueError, "pull-refused"):
                        await environment._run_docker_compose_command(["pull"])
                self.assertEqual(len(inspections), 2)
                self.assertTrue(all(args[-1] == "native-cached-image" for args in inspections))

    async def test_missing_cached_verifier_image_prevents_start(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                environment = module._VerifierDockerEnvironment(Path(root))
                environment._env_vars = types.SimpleNamespace(main_image_name="missing")
                environment.task_env_config = types.SimpleNamespace(docker_image="missing")
                environment._use_prebuilt = True
                async def missing(*args):
                    raise ValueError("cached-verifier-image-unavailable")
                with patch.object(module, "_regrade_docker", missing):
                    for command in (["build"], ["up", "--detach", "--wait"]):
                        with self.assertRaisesRegex(ValueError, "image-unavailable"):
                            await environment._run_docker_compose_command(command)
                self.assertEqual(environment.commands, [])

    async def test_task_manifest_uses_relative_string_order(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                root = Path(root)
                (root / "git").mkdir()
                (root / "git-command").write_bytes(b"wrapper")
                (root / "git/runtime").write_bytes(b"runtime")
                rows = (
                    f"git-command\n{hashlib.sha256(b'wrapper').hexdigest()}\n"
                    f"git/runtime\n{hashlib.sha256(b'runtime').hexdigest()}\n"
                )
                self.assertEqual(module._regrade_manifest(root), hashlib.sha256(rows.encode()).hexdigest())

    async def test_restored_bytes_bind_file_and_directory_contents(self):
        for family, (module, _) in INTEGRATIONS.items():
            with self.subTest(family=family), tempfile.TemporaryDirectory() as root:
                root = Path(root)
                answer = root / "answer"
                answer.write_bytes(b"correct\n")
                expected = hashlib.sha256(b"correct\n").hexdigest()
                self.assertEqual(module._regrade_artifact(answer), expected)
                self.assertEqual(module._regrade_artifact(root), hashlib.sha256(f"answer\\n{expected}".encode()).hexdigest())
                answer.write_bytes(b"incorrect\n")
                self.assertNotEqual(module._regrade_artifact(answer), expected)
                if os.name == "posix":
                    answer.with_name("link").symlink_to(answer)
                    with self.assertRaisesRegex(ValueError, "unsafe-verifier-input"):
                        module._regrade_artifact(root)


class FakeEnvironment:
    """Async file-transfer/exec boundary with no shell, container, or provider."""

    def __init__(self):
        self.capabilities = types.SimpleNamespace(mounted=False)
        self.files = {}
        self.directories = set()
        self.uploads = []
        self.launches = []
        self.cleaned = False
        self.launch_code = 0
        self.launch_error = None
        self.cleanup_error = None
        self.cleanup_code = 0
        self.download_error = None
        self.setup_code = 0
        self.proxy_env = {}
        self.proxy_inputs = []
        self.exec_envs = []
        self.preflight_calls = []
        self.preflight_code = 0
        self.preflight_stdout = "0.85.1\n"
        self.preflight_stderr = ""
        self.preflight_error = None

    def agent_process_env(self, env):
        self.proxy_inputs.append(dict(env))
        return {**self.proxy_env, **env}

    async def upload_file(self, source_path, target_path):
        target = str(target_path)
        if str(PurePosixPath(target).parent) not in self.directories:
            raise RuntimeError("upload before awaited mkdir")
        self.uploads.append(target)
        self.files[target] = Path(source_path).read_bytes()

    async def upload_dir(self, source_dir, target_dir):
        self.uploads.append(str(target_dir))
        for path in Path(source_dir).rglob("*"):
            if path.is_file():
                self.files[f"{target_dir}/{path.relative_to(source_dir).as_posix()}"] = path.read_bytes()

    async def download_file(self, source_path, target_path):
        if self.cleaned:
            raise RuntimeError("evidence downloaded after cleanup")
        if self.download_error:
            raise self.download_error
        target = Path(target_path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(self.files[str(source_path)])

    async def download_dir(self, source_dir, target_dir):
        for source in list(self.files):
            if source.startswith(f"{source_dir}/"):
                await self.download_file(source, Path(target_dir) / source[len(str(source_dir)) + 1:])

    async def is_dir(self, path, user=None):
        return any(source.startswith(f"{path}/") for source in self.files)

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None):
        self.exec_envs.append((command, env))
        args = shlex.split(command)
        if args[:2] == ["bash", "-c"] and args[2].startswith("set -m\n"):
            args = shlex.split(args[2].splitlines()[1].removesuffix(" &"))
        result = types.SimpleNamespace(return_code=0, stdout="", stderr="")
        if args[-1] == "--version":
            self.preflight_calls.append((command, env, timeout_sec, user))
            if self.preflight_error:
                raise self.preflight_error
            return types.SimpleNamespace(return_code=self.preflight_code,
                                         stdout=self.preflight_stdout, stderr=self.preflight_stderr)
        if len(args) >= 3 and args[1:3] == ["__native-capture", "--selfcheck"]:
            self.preflight_calls.append((command, env, timeout_sec, user))
            return types.SimpleNamespace(
                return_code=0,
                stdout=json.dumps({
                    "schema": "opi-eval-native-capture-helper/1",
                    "protocol": "opi-eval-native-capture/1",
                    "policies": ["full", "terminal-only"],
                }) + "\n",
                stderr="",
            )
        if len(args) >= 2 and args[1] == "__native-capture":
            # The helper launch: model a sanitized capture with a receipt
            # that binds the retained bytes (P19-AUD-003).
            def option(name):
                return args[args.index(name) + 1]
            policy = option("--policy")
            stdout_target = option("--stdout")
            stderr_target = option("--stderr")
            receipt_target = option("--receipt")
            self.files[stdout_target] = STDOUT
            self.files[stderr_target] = STDERR
            self.files[receipt_target] = (
                json.dumps({
                    "schema": "opi-eval-native-capture/1",
                    "policy": policy,
                    "status": "sanitized",
                    "agent_exit_code": self.launch_code,
                    "cancelled": False,
                    "refusal_reason": None,
                    "stdout": {
                        "bytes": len(STDOUT),
                        "sha256": hashlib.sha256(STDOUT).hexdigest(),
                        "truncated": False,
                    },
                    "stderr": {
                        "status": "opaque" if STDERR else "empty",
                        "bytes": len(STDERR),
                        "sha256": hashlib.sha256(STDERR).hexdigest(),
                    },
                }) + "\n"
            ).encode()
        if args[0] == "mkdir":
            if self.setup_code:
                result.return_code = self.setup_code
                return result
            self.directories.add(args[2])
        elif args[0] == "chmod":
            pass
        elif args[0] == "rm":
            if self.cleanup_error:
                raise self.cleanup_error
            result.return_code = self.cleanup_code
            if not self.cleanup_code:
                self.cleaned = True
                self.files.clear()
        elif command.startswith("pwd && find"):
            result.stdout = "/work\n" + "a" * 64 + "  ./answer.txt\n"
        else:
            self.launches.append((args, env))
            stage = "/opi-eval-agent/pi" if "--provider" in args else "/opi-eval-agent/opi"
            if "--trace" in args:
                self.files[f"{stage}/trace/run-1/evidence.jsonl"] = b"trace\n"
                self.files[f"{stage}/trace/run-1/manifest.json"] = b"{}\n"
            if self.launch_error:
                raise self.launch_error
            result.return_code = self.launch_code
            result.stdout = STDOUT.decode()
            result.stderr = STDERR.decode()
        return result


class LocalShellEnvironment:
    """Linux environment fake: real shell and copies within one temp directory."""

    def __init__(self, root):
        self.capabilities = types.SimpleNamespace(mounted=False)
        self.stage_root = root / "stage"
        self.work = root / "work"
        self.work.mkdir(parents=True)
        self.fail_config_upload = False
        self.config_upload_error = RuntimeError("fixture upload failed after partial config write")

    def agent_process_env(self, env):
        return env

    def local(self, remote):
        relative = PurePosixPath(remote).relative_to("/opi-eval-agent")
        if ".." in relative.parts:
            raise ValueError("test environment path escape")
        return self.stage_root.joinpath(*relative.parts)

    async def upload_file(self, source_path, target_path):
        shutil.copyfile(source_path, self.local(target_path))
        if self.fail_config_upload and str(target_path).endswith("models.json"):
            raise self.config_upload_error

    async def upload_dir(self, source_dir, target_dir):
        shutil.copytree(source_dir, self.local(target_dir))

    async def download_file(self, source_path, target_path):
        shutil.copyfile(self.local(source_path), target_path)

    async def download_dir(self, source_dir, target_dir):
        shutil.copytree(self.local(source_dir), target_dir)

    async def is_dir(self, path, user=None):
        return self.local(path).is_dir()

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None):
        # Only the fixed environment stage prefix is translated; all shell
        # operators, argument quoting and redirections execute unchanged.
        result = subprocess.run(
            ["/bin/sh", "-c", command.replace("/opi-eval-agent", shlex.quote(str(self.stage_root)))],
            cwd=self.work,
            env={"PATH": "/usr/bin:/bin", "HOME": str(self.work), **(env or {})},
            capture_output=True, text=True, timeout=10,
        )
        return types.SimpleNamespace(
            return_code=result.returncode, stdout=result.stdout, stderr=result.stderr,
        )


class MountedShellEnvironment(LocalShellEnvironment):
    """Pinned runners' task-owned /logs/agent mount, with a real child shell."""

    def __init__(self, root, logs):
        super().__init__(root)
        self.logs = logs
        self.capabilities = types.SimpleNamespace(mounted=True)

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None):
        command = command.replace("/opi-eval-agent", shlex.quote(str(self.stage_root)))
        command = command.replace("/logs/agent", shlex.quote(str(self.logs)))
        proc = await asyncio.create_subprocess_exec(
            "/bin/sh", "-c", command, cwd=self.work,
            env={"PATH": "/usr/bin:/bin", "HOME": str(self.work), **(env or {})},
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        return types.SimpleNamespace(return_code=proc.returncode,
                                     stdout=stdout.decode(), stderr=stderr.decode())


class DockerCaptureEnvironment:
    """Same exec/upload seam on one new, network-disabled Docker container."""

    def __init__(self, container):
        self.container = container
        self.capabilities = types.SimpleNamespace(mounted=True)

    def agent_process_env(self, env):
        return env

    async def upload_file(self, source_path, target_path):
        proc = await asyncio.create_subprocess_exec(
            "docker", "cp", str(source_path), f"{self.container}:{target_path}",
            stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        _, stderr = await proc.communicate()
        if proc.returncode:
            raise RuntimeError(stderr.decode())

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None):
        args = ["docker", "exec", "--workdir", "/work"]
        for key, value in (env or {}).items():
            args.extend(["--env", f"{key}={value}"])
        args.extend([self.container, "/bin/sh", "-c", command])
        proc = await asyncio.create_subprocess_exec(
            *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()
        return types.SimpleNamespace(return_code=proc.returncode,
                                     stdout=stdout.decode(), stderr=stderr.decode())


async def capture_child(root, family, product, container=None):
    """Synthetic Agent makes progress, then never returns to the integration."""
    logs = root / "logs"
    logs.mkdir(exist_ok=True)
    executable = root / "client"
    executable.write_text(
        "#!/bin/sh\n"
        "if [ \"$1\" = --version ]; then printf '0.85.1\\n'; exit 0; fi\n"
        "while [ $# -gt 0 ]; do\n"
        "  if [ \"$1\" = --trace ]; then shift; mkdir -p \"$1/run-1\"; "
        "printf 'trace-progress\\n' > \"$1/run-1/evidence.jsonl\"; fi\n"
        "  shift\n"
        "done\n"
        # P19-AUD-003: the emitter's stream must be admissible at the
        # first sink; a safe record prefix survives the outer kill.
        "printf '%s\\n' 'CAPTURE_SAFE_RECORD'\n"
        "touch ready\n"
        "sleep 300\n", encoding="utf-8",
    )
    if product == "pi":
        safe_prefix = (
            '{"type":"session","version":2,"id":"capture-child","timestamp":"t","cwd":"/work"}\n'
            '{"type":"tool_execution_start","toolCallId":"c1"}'
        )
    else:
        safe_prefix = '{"type":"Agent","event":{"type":"AgentStart"}}'
    executable.write_text(executable.read_text().replace("CAPTURE_SAFE_RECORD", safe_prefix))
    handoff = {
        "schema": "opi-eval-native-handoff/4", "product": product, "trial": "capture-child",
        "integration_agent_name": f"opi-eval-{product}",
        "agent_executable": str(executable),
        "agent_executable_sha256": hashlib.sha256(executable.read_bytes()).hexdigest(),
        "capture_helper": capture_helper_block(),
        "capture_policy": "full",
        "model": "scripted:fixture", "inference_endpoint": "http://localhost:8080/v1",
        "agent_env": {},
    }
    (root / ("config.toml" if product == "opi" else "models.json")).write_text(
        "synthetic-private-config-canary", encoding="utf-8",
    )
    agent = NativeIntegrationTest.agent(None, family, root, logs, handoff)
    # Harbor's BaseAgent owns this public path; Pier uses EnvironmentPaths.
    agent.environment_logs_dir = PurePosixPath("/logs/agent")
    environment = (DockerCaptureEnvironment(container) if container
                   else MountedShellEnvironment(root / "environment", logs))
    await agent.setup(environment)
    await agent.run("synthetic work", environment, object())


class NativeIntegrationTest(unittest.IsolatedAsyncioTestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def prepare(self, family, product="pi", package=True):
        root = Path(tempfile.mkdtemp(prefix=f"{family}-{product}-", dir=self.root))
        runtime = root / "release"
        for name, data in PACKAGE_FILES.items():
            path = runtime / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        config = "models.json" if product == "pi" else "config.toml"
        (root / config).write_text("projected-config-only", encoding="utf-8")
        (root / "unbound-neighbor").write_bytes(b"must not upload")
        handoff = {
            "schema": "opi-eval-native-handoff/4",
            "trial": "fixture-native-trial",
            "product": product,
            "integration_agent_name": f"opi-eval-{product}",
            "agent_executable": str(runtime / "pi"),
            "agent_executable_sha256": AGENT_SHA,
            "capture_helper": capture_helper_block(),
            "capture_policy": "full",
            "model": "zhipu:glm-5.3-flash",
            "inference_endpoint": "https://api.selected.example/api/v4",
            "config_path": config,
            "agent_env": {"TEST_PROJECTED": "yes"},
        }
        if package:
            handoff["runtime_package"] = {
                "path": str(runtime), "manifest_sha256": PACKAGE_SHA, "entrypoint": "pi",
            }
        logs = root / "logs"
        logs.mkdir()
        return root, runtime, logs, handoff

    def agent(self, family, root, logs, handoff):
        path = root / "handoff.json"
        path.write_text(json.dumps(handoff), encoding="utf-8")
        module, _ = INTEGRATIONS[family]
        cls = module.PiAgent if handoff["product"] == "pi" else module.OpiAgent
        with patch.dict(os.environ, {"OPI_EVAL_NATIVE_HANDOFF": str(path)}):
            return cls(logs_dir=logs)

    @unittest.skipUnless(os.name == "posix", "requires Linux process groups and shell")
    async def test_outer_kill_preserves_partial_capture_without_terminal(self):
        await self.outer_kill_capture()

    @unittest.skipUnless(os.name == "posix" and "--docker" in sys.argv,
                         "opt in with --docker; requires the cached Python witness image")
    async def test_docker_outer_kill_preserves_partial_capture_without_terminal(self):
        await self.outer_kill_capture(docker=True)

    async def outer_kill_capture(self, docker=False):
        for family in INTEGRATIONS:
            for product in ("pi", "opi"):
                with self.subTest(family=family, product=product):
                    root = Path(tempfile.mkdtemp(dir=self.root))
                    container = None
                    if docker:
                        (root / "logs").mkdir()
                        started = subprocess.run([
                            "docker", "run", "--detach", "--network", "none", "--pull", "never",
                            "--mount", f"type=bind,source={root / 'logs'},target=/logs/agent",
                            "--label", f"opi-eval-capture-fixture={root}",
                            "python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc",
                            "sh", "-c", "mkdir /work; sleep 300",
                        ], check=True, capture_output=True, text=True, timeout=30)
                        container = started.stdout.strip()
                        self.assertRegex(container, r"^[a-f0-9]{64}$")
                    proc = await asyncio.create_subprocess_exec(
                        sys.executable, str(Path(__file__).resolve()), "--capture-child",
                        str(root), family, product, *([container] if container else []), start_new_session=True,
                        env={"PATH": os.environ["PATH"], "PYTHONDONTWRITEBYTECODE": "1"},
                        stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
                    )
                    try:
                        async with asyncio.timeout(15):
                            while True:
                                if container:
                                    ready = await DockerCaptureEnvironment(container).exec("test -f /work/ready")
                                    if ready.return_code == 0:
                                        break
                                elif (root / "environment/work/ready").exists():
                                    break
                                if proc.returncode is not None:
                                    self.fail((await proc.communicate())[1].decode())
                                await asyncio.sleep(0.02)
                    finally:
                        # Model owned-container removal for the local fixture:
                        # its Agent has a separate process group inside this
                        # test's private environment.
                        pid_file = root / "environment/stage" / product / "agent.pid"
                        if not container and pid_file.is_file():
                            child_pid = int(pid_file.read_text())
                            try:
                                os.killpg(child_pid, signal.SIGKILL)
                            except ProcessLookupError:
                                pass
                        if proc.returncode is None:
                            os.killpg(proc.pid, signal.SIGKILL)
                        await proc.communicate()
                        if container:
                            # The fixture knows every created path and creates no
                            # links. Restore host ownership solely for temp cleanup.
                            subprocess.run(["docker", "exec", container, "chown", "-R",
                                            f"{os.getuid()}:{os.getgid()}", "/logs/agent"],
                                           check=True, capture_output=True, timeout=10)
                            subprocess.run(["docker", "rm", "--force", container],
                                           check=True, capture_output=True, timeout=30)
                            gone = subprocess.run(["docker", "inspect", container],
                                                  capture_output=True, timeout=10)
                            self.assertNotEqual(gone.returncode, 0)
                    # Like owned-container removal, deletes private staging after
                    # the runner and its children are dead; no Python finally ran.
                    shutil.rmtree(root / "environment/stage", ignore_errors=True)
                    logs = root / "logs"
                    def safe_record(product):
                        if product == "pi":
                            return (b'{"type":"session","version":2,"id":"capture-child",'
                                    b'"timestamp":"t","cwd":"/work"}\n'
                                    b'{"type":"tool_execution_start","toolCallId":"c1"}\n')
                        return b'{"type":"Agent","event":{"type":"AgentStart"}}\n'


                    self.assertTrue((logs / "stdout.log").exists(), "outer kill lost Agent stdout")
                    self.assertEqual((logs / "stdout.log").read_bytes(), safe_record(product))
                    self.assertEqual((logs / "stderr.log").read_bytes(), b"")
                    if product == "opi":
                        self.assertEqual((logs / "trace/run-1/evidence.jsonl").read_bytes(), b"trace-progress\n")
                    self.assertFalse((logs / "opi-eval-terminal.json").exists())
                    self.assertFalse((root / "environment/stage").exists())
                    for path in logs.rglob("*"):
                        if path.is_file():
                            self.assertNotIn(b"synthetic-private-config-canary", path.read_bytes())

    async def test_selected_thinking_level_is_launched_and_invalid_level_refused(self):
        for family in INTEGRATIONS:
            for level in ("off", "minimal", "low", "medium", "high", "xhigh", "max"):
                with self.subTest(family=family, level=level):
                    root, _, logs, handoff = self.prepare(family, "pi", False)
                    handoff["thinking_level"] = level
                    path = root / "handoff.json"
                    path.write_text(json.dumps(handoff))
                    agent = self.agent(family, root, logs, handoff)
                    environment = FakeEnvironment()
                    await agent.setup(environment)
                    await agent.run("task", environment, object())
                    args, _ = environment.launches[0]
                    self.assertEqual(args[args.index("--thinking") + 1], level)
            root, _, logs, handoff = self.prepare(family, "pi", False)
            handoff["thinking_level"] = "ultra; injected"
            with self.assertRaisesRegex(Exception, "thinking"):
                self.agent(family, root, logs, handoff)._agent_command("task", PurePosixPath("/stage"))

    async def test_package_and_single_binary_launches_retain_evidence(self):
        for family in INTEGRATIONS:
            for product, package in (("pi", True), ("pi", False), ("opi", False)):
                with self.subTest(family=family, product=product):
                    root, runtime, logs, handoff = self.prepare(family, product, package)
                    agent = self.agent(family, root, logs, handoff)
                    environment = FakeEnvironment()
                    await agent.setup(environment)
                    stage = f"/opi-eval-agent/{product}"
                    if package:
                        self.assertEqual(environment.files[f"{stage}/runtime/theme/dark.json"], b"dark")
                        self.assertEqual(environment.files[f"{stage}/runtime/package.json"], b"{}")
                        self.assertEqual(
                environment.uploads,
                [f"{stage}/runtime", f"{stage}/capture-helper", f"{stage}/models.json"],
            )
                    else:
                        config = "models.json" if product == "pi" else "config.toml"
                        self.assertEqual(
                environment.uploads,
                [f"{stage}/agent", f"{stage}/capture-helper", f"{stage}/{config}"],
            )
                    prompt = "--unsafe 'literal'; $(not-a-command)"
                    await agent.run(prompt, environment, object())
                    args, env = environment.launches[0]
                    self.assertEqual(args[len(args) - 1 - args[::-1].index("--") + 1], prompt)
                    self.assertIn("__native-capture", args)
                    expected_env = {"TEST_PROJECTED": "yes"}
                    if product == "opi":
                        expected_env["DBUS_SESSION_BUS_ADDRESS"] = HEADLESS_BUS
                    self.assertEqual(env, expected_env)
                    if product == "pi":
                        self.assertIn(f"{stage}/runtime/pi" if package else f"{stage}/agent", args)
                        self.assertEqual(args[args.index("--provider") + 1], "zhipu")
                        self.assertEqual(args[args.index("--model") + 1], "glm-5.3-flash")
                        self.assertEqual(args[args.index("--thinking") + 1], "off")
                        for flag in ("--no-approve", "--no-context-files", "--no-extensions", "--no-skills"):
                            self.assertIn(flag, args)
                    else:
                        self.assertEqual((logs / "trace/run-1/evidence.jsonl").read_bytes(), b"trace\n")
                    self.assertEqual((logs / "stdout.log").read_bytes(), STDOUT)
                    self.assertEqual((logs / "stderr.log").read_bytes(), STDERR)
                    terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                    self.assertEqual(terminal["schema"], "opi-eval-native-agent-terminal/3")
                    self.assertEqual(terminal["trial"], handoff["trial"])
                    self.assertEqual(terminal["capture"]["status"], "sanitized")
                    self.assertTrue(terminal["task_dispatched"])
                    self.assertEqual(terminal["runtime_preflight"]["status"], "passed")
                    self.assertEqual(terminal["agent_executable_sha256"], AGENT_SHA)
                    self.assertEqual(terminal["delivered"], [{"source": "/work/answer.txt", "sha256": "a" * 64}])
                    self.assertTrue(environment.cleaned)

    async def test_runtime_preflight_binds_command_captures_and_no_secret_projection(self):
        for family in INTEGRATIONS:
            for product, package in (("opi", False), ("pi", True)):
                with self.subTest(family=family, product=product):
                    root, _, logs, handoff = self.prepare(family, product, package)
                    environment = FakeEnvironment()
                    agent = self.agent(family, root, logs, handoff)
                    await agent.setup(environment)
                    self.assertEqual(environment.launches, [])
                    # Two preflights now run: the staged runtime version and
                    # the capture helper self-check (P19-AUD-003).
                    self.assertEqual(len(environment.preflight_calls), 2)
                    command, env, timeout, user = environment.preflight_calls[0]
                    executable = f"/opi-eval-agent/{product}/runtime/pi" if package else f"/opi-eval-agent/{product}/agent"
                    self.assertEqual(command, shlex.quote(executable) + " --version")
                    self.assertEqual(timeout, 30)
                    self.assertIsNone(user)
                    self.assertNotIn("TEST_PROJECTED", env)
                    self.assertNotIn("HTTPS_PROXY", env)
                    self.assertTrue(all(value.startswith(f"/opi-eval-agent/{product}/")
                                        or value.startswith("unix:abstract=") for value in env.values()))
                    await agent.run("work", environment, object())
                    terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                    helper_proof = terminal["helper_preflight"]
                    self.assertEqual(helper_proof["kind"], "capture-helper-selfcheck")
                    self.assertEqual(helper_proof["status"], "passed")
                    self.assertEqual(helper_proof["exit_code"], 0)
                    helper_command, _, helper_timeout, _ = environment.preflight_calls[1]
                    self.assertIn("__native-capture --selfcheck", helper_command)
                    self.assertEqual(
                        helper_proof["command_sha256"],
                        hashlib.sha256(helper_command.encode()).hexdigest(),
                    )
                    self.assertEqual(helper_timeout, 30)
                    proof = terminal["runtime_preflight"]
                    self.assertEqual(proof["kind"], "version")
                    self.assertEqual(proof["status"], "passed")
                    self.assertEqual(proof["exit_code"], 0)
                    self.assertEqual(proof["agent_executable_sha256"], AGENT_SHA)
                    self.assertEqual(proof["runtime_package_manifest_sha256"], PACKAGE_SHA if package else None)
                    self.assertEqual(proof["runtime_package_entrypoint"], "pi" if package else None)
                    self.assertEqual(proof["command_sha256"], hashlib.sha256(command.encode()).hexdigest())
                    for stream in ("stdout", "stderr"):
                        raw = (logs / f"runtime-preflight-{stream}.log").read_bytes()
                        self.assertEqual(proof[f"{stream}_sha256"], hashlib.sha256(raw).hexdigest())
                        self.assertTrue(proof[f"{stream}_complete"])
                        self.assertFalse(proof[f"{stream}_truncated"])

    async def test_runtime_preflight_refusals_never_dispatch_or_project_credentials(self):
        cases = (
            ("nonzero", 1, b"", b"loader refused\n", "failed", None),
            ("empty", 0, b"", b"", "unknown", "invalid-output"),
            ("missing", 0, None, b"", "unknown", "invalid-output"),
            ("invalid", 0, bytes([255]), b"", "unknown", "invalid-output"),
            ("overflow", 0, b"123456789", b"", "unknown", "capture-overflow"),
            ("missing-exit", None, b"version\n", b"", "unknown", "invalid-output"),
        )
        for family, (module, _) in INTEGRATIONS.items():
            for product in ("opi", "pi"):
                for case, code, stdout, stderr, status, reason in cases:
                    with self.subTest(family=family, product=product, case=case):
                        root, _, logs, handoff = self.prepare(family, product)
                        environment = FakeEnvironment()
                        environment.preflight_code = code
                        environment.preflight_stdout = stdout
                        environment.preflight_stderr = stderr
                        with patch.object(module, "_PREFLIGHT_CAPTURE_BYTES", 8 if case == "overflow" else 64):
                            with self.assertRaisesRegex(RuntimeError, f"runtime preflight {status}"):
                                await self.agent(family, root, logs, handoff).setup(environment)
                        terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                        self.assertFalse(terminal["task_dispatched"])
                        self.assertIsNone(terminal["exit_code"])
                        self.assertFalse(terminal["timed_out"])
                        self.assertEqual(terminal["delivered"], [])
                        self.assertEqual(terminal["runtime_preflight"]["status"], status)
                        self.assertEqual(terminal["runtime_preflight"]["reason"], reason)
                        self.assertEqual(environment.launches, [])
                        self.assertTrue(environment.cleaned)
                        self.assertFalse(any(path.endswith(("/config.toml", "/models.json"))
                                             for path in environment.uploads))
                        self.assertNotIn("TEST_PROJECTED", environment.preflight_calls[0][1])
                        if case == "overflow":
                            proof = terminal["runtime_preflight"]
                            self.assertTrue(proof["stdout_truncated"])
                            self.assertFalse(proof["stdout_complete"])
                            self.assertEqual((logs / "runtime-preflight-stdout.log").read_bytes(), stdout)
                            self.assertEqual(proof["stdout_sha256"], hashlib.sha256(stdout[:8]).hexdigest())

    async def test_runtime_preflight_transport_timeout_and_cancellation_remain_unknown(self):
        for family in INTEGRATIONS:
            for product in ("opi", "pi"):
                for error, reason in ((TimeoutError("deadline"), "timeout"),
                                      (RuntimeError("transport failed"), "transport"),
                                      (INTEGRATIONS[family][0]._NativeCommandTerminationError("unconfirmed"),
                                       "termination"),
                                      (asyncio.CancelledError("cancelled"), "termination")):
                    with self.subTest(family=family, product=product, reason=reason):
                        root, _, logs, handoff = self.prepare(family, product)
                        environment = FakeEnvironment()
                        environment.preflight_error = error
                        with self.assertRaises(type(error)):
                            await self.agent(family, root, logs, handoff).setup(environment)
                        terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                        self.assertEqual(terminal["runtime_preflight"]["status"], "unknown")
                        self.assertEqual(terminal["runtime_preflight"]["reason"], reason)
                        self.assertIsNone(terminal["runtime_preflight"]["exit_code"])
                        self.assertFalse(terminal["task_dispatched"])
                        self.assertIsNone(terminal["exit_code"])
                        self.assertFalse(terminal["timed_out"])
                        self.assertEqual(environment.launches, [])
                        self.assertTrue(environment.cleaned)

    async def test_missing_or_unsafe_trial_identity_rejects_before_upload(self):
        for family in INTEGRATIONS:
            for trial in (None, "", "../trial", "bad/trial", "trial\n"):
                with self.subTest(family=family, trial=trial):
                    root, _, logs, handoff = self.prepare(family)
                    handoff["trial"] = trial
                    with self.assertRaisesRegex(RuntimeError, "trial identity"):
                        self.agent(family, root, logs, handoff)

    async def test_runtime_identity_drift_after_preflight_prevents_task_dispatch(self):
        for family in INTEGRATIONS:
            for change in ("executable", "resource"):
                with self.subTest(family=family, change=change):
                    root, runtime, logs, handoff = self.prepare(family)
                    environment = FakeEnvironment()
                    agent = self.agent(family, root, logs, handoff)
                    await agent.setup(environment)
                    (runtime / ("pi" if change == "executable" else "theme/dark.json")).write_bytes(b"changed")
                    with self.assertRaisesRegex(RuntimeError, "digest mismatch"):
                        await agent.run("never dispatch changed runtime", environment, object())
                    self.assertEqual(environment.launches, [])
                    self.assertTrue(environment.cleaned)
                    self.assertFalse((logs / "opi-eval-terminal.json").exists())

    @unittest.skipUnless(os.name == "posix", "requires Linux symlinks")
    async def test_runtime_capture_does_not_follow_existing_symlink(self):
        for family in INTEGRATIONS:
            root, _, logs, handoff = self.prepare(family)
            neighbor = root / "neighbor"
            neighbor.write_bytes(b"unrelated")
            (logs / "runtime-preflight-stdout.log").symlink_to(neighbor)
            environment = FakeEnvironment()
            with self.assertRaises(FileExistsError):
                await self.agent(family, root, logs, handoff).setup(environment)
            self.assertEqual(neighbor.read_bytes(), b"unrelated")
            self.assertEqual(environment.launches, [])
            self.assertTrue(environment.cleaned)

    async def test_rejects_runtime_drift_before_any_upload(self):
        for family in INTEGRATIONS:
            for case in ("executable", "manifest", "entrypoint"):
                with self.subTest(family=family, case=case):
                    root, runtime, logs, handoff = self.prepare(family)
                    if case == "executable":
                        handoff["agent_executable_sha256"] = "0" * 64
                    elif case == "manifest":
                        (runtime / "theme/light.json").write_bytes(b"tampered")
                    else:
                        handoff["runtime_package"]["entrypoint"] = "theme/dark.json"
                    agent = self.agent(family, root, logs, handoff)
                    environment = FakeEnvironment()
                    with self.assertRaisesRegex(RuntimeError, f"{case}.*digest mismatch"):
                        await agent.setup(environment)
                    self.assertEqual(environment.uploads, [])

    async def test_rejects_unsafe_package_entrypoints_before_upload(self):
        for family in INTEGRATIONS:
            for entrypoint in ("../pi", "/pi", "C:/pi", "a\\pi", "./pi", "a//pi", "pi\n", "", "."):
                with self.subTest(family=family, entrypoint=entrypoint):
                    root, _, logs, handoff = self.prepare(family)
                    handoff["runtime_package"]["entrypoint"] = entrypoint
                    agent = self.agent(family, root, logs, handoff)
                    environment = FakeEnvironment()
                    with self.assertRaisesRegex(RuntimeError, "unsafe runtime package"):
                        await agent.setup(environment)
                    self.assertEqual(environment.uploads, [])

    async def test_missing_entrypoint_and_relative_package_directory_reject(self):
        for family in INTEGRATIONS:
            for field, value in (("entrypoint", "missing"), ("path", "relative-package")):
                with self.subTest(family=family, field=field):
                    root, _, logs, handoff = self.prepare(family)
                    handoff["runtime_package"][field] = value
                    environment = FakeEnvironment()
                    with self.assertRaisesRegex(RuntimeError, "runtime package"):
                        await self.agent(family, root, logs, handoff).setup(environment)
                    self.assertEqual(environment.uploads, [])

    @unittest.skipUnless(os.name == "posix", "Linux native package path rules")
    async def test_symlinks_and_special_files_reject_before_upload(self):
        for family in INTEGRATIONS:
            for kind in ("file-link", "directory-link", "root-link", "fifo", "newline-name", "backslash-name"):
                with self.subTest(family=family, kind=kind):
                    root, runtime, logs, handoff = self.prepare(family)
                    unexpected = runtime / "unexpected"
                    if kind == "file-link":
                        unexpected.symlink_to(root / "unbound-neighbor")
                    elif kind == "directory-link":
                        unexpected.symlink_to(root, target_is_directory=True)
                    elif kind == "root-link":
                        (root / "linked").symlink_to(runtime, target_is_directory=True)
                        handoff["runtime_package"]["path"] = str(root / "linked")
                    elif kind == "fifo":
                        os.mkfifo(unexpected)
                    else:
                        unexpected = runtime / ("line\nbreak" if kind == "newline-name" else "back\\slash")
                        unexpected.write_bytes(b"unbound")
                    environment = FakeEnvironment()
                    reason = (
                        "unsafe runtime package directory" if kind == "root-link"
                        else "unsafe runtime package path" if kind in ("newline-name", "backslash-name")
                        else "runtime package contains a symlink or special file"
                    )
                    with self.assertRaisesRegex(RuntimeError, reason):
                        await self.agent(family, root, logs, handoff).setup(environment)
                    self.assertEqual(environment.uploads, [])
                    if kind == "root-link":
                        (root / "linked").unlink()
                    else:
                        unexpected.unlink()

    async def test_malformed_connection_and_unbound_config_reject_before_upload(self):
        for family in INTEGRATIONS:
            for field, value in (("model", "zhipu"), ("model", ":model"),
                                 ("model", "zhipu:"), ("model", " :model"),
                                 ("config_path", "unbound-neighbor")):
                with self.subTest(family=family, field=field, value=value):
                    root, _, logs, handoff = self.prepare(family)
                    handoff[field] = value
                    environment = FakeEnvironment()
                    with self.assertRaisesRegex(RuntimeError, "handoff (model|configuration)"):
                        await self.agent(family, root, logs, handoff).setup(environment)
                    self.assertEqual(environment.uploads, [])

    async def test_old_handoff_schema_rejects(self):
        for family in INTEGRATIONS:
            root, _, logs, handoff = self.prepare(family)
            handoff["schema"] = "opi-eval-native-handoff/3"
            with self.assertRaisesRegex(RuntimeError, "handoff schema"):
                self.agent(family, root, logs, handoff)

    async def test_agent_failures_retain_partial_logs_and_terminal_cause(self):
        for family in INTEGRATIONS:
            for failure in ("exit-code", "timeout", "cancelled"):
                with self.subTest(family=family, failure=failure):
                    root, _, logs, handoff = self.prepare(family)
                    environment = FakeEnvironment()
                    if failure == "cancelled":
                        environment.launch_error = asyncio.CancelledError("fixture cancellation")
                    elif failure == "timeout":
                        environment.launch_error = TimeoutError("fixture timeout")
                    else:
                        environment.launch_code = 7
                    agent = self.agent(family, root, logs, handoff)
                    await agent.setup(environment)
                    with patch.dict(sys.modules, INTEGRATIONS[family][1]):
                        error = (asyncio.CancelledError if failure == "cancelled"
                                 else TimeoutError if failure == "timeout" else NonZeroAgentExitCodeError)
                        with self.assertRaises(error):
                            await agent.run("work", environment, object())
                    self.assertEqual((logs / "stdout.log").read_bytes(), STDOUT)
                    self.assertEqual((logs / "stderr.log").read_bytes(), STDERR)
                    self.assertTrue(environment.cleaned)
                    if failure == "cancelled":
                        self.assertFalse((logs / "opi-eval-terminal.json").exists())
                        continue
                    terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                    self.assertEqual(terminal["exit_code"], {"exit-code": 7, "timeout": None}[failure])
                    self.assertEqual(terminal["timed_out"], failure == "timeout")
                    self.assertTrue(environment.cleaned)

    @unittest.skipUnless(os.name == "posix", "requires Linux process groups and shell")
    async def test_stage_cancellation_observes_exit_and_stops_descendants(self):
        for family in INTEGRATIONS:
            for product in ("opi", "pi"):
                with self.subTest(family=family, product=product):
                    root, runtime, logs, handoff = self.prepare(family, product, package=False)
                    executable = runtime / "pi"
                    safe_prefix = (
                        '{"type":"session","version":2,"id":"cancel-stage","timestamp":"t","cwd":"/work"}'
                        if product == "pi"
                        else '{"type":"Agent","event":{"type":"AgentStart"}}'
                    )
                    executable.write_text(
                        "#!/bin/sh\n"
                        "if [ \"$1\" = --version ]; then printf '0.85.1\\n'; exit 0; fi\n"
                        f"printf '%s\\n' '{safe_prefix}'\n"
                        "(sleep 2; touch escaped) &\n"
                        "wait\n", encoding="utf-8",
                    )
                    handoff["agent_executable_sha256"] = hashlib.sha256(executable.read_bytes()).hexdigest()
                    agent = self.agent(family, root, logs, handoff)
                    environment = MountedShellEnvironment(root / "environment", logs)
                    await agent.setup(environment)
                    with self.assertRaises(TimeoutError):
                        await asyncio.wait_for(agent.run("work", environment, object()), timeout=0.3)
                    terminal_path = logs / "opi-eval-terminal.json"
                    observed = json.loads(terminal_path.read_text()) if terminal_path.exists() else None
                    # Allow an escaped child to expose itself before temp cleanup.
                    await asyncio.sleep(2)
                    self.assertFalse((environment.work / "escaped").exists())
                    self.assertIsNotNone(observed, "stage cancellation lost observed terminal identity")
                    self.assertEqual(observed["exit_code"], 137)
                    self.assertFalse(observed["timed_out"], "only the runner knows cancellation's cause")
                    self.assertEqual(observed["agent_executable_sha256"], handoff["agent_executable_sha256"])
                    self.assertFalse(environment.local(f"/opi-eval-agent/{product}").exists())

    async def test_cleanup_failure_never_publishes_success(self):
        for family in INTEGRATIONS:
            for failure in ("exception", "exit-code"):
                with self.subTest(family=family, failure=failure):
                    root, _, logs, handoff = self.prepare(family)
                    environment = FakeEnvironment()
                    agent = self.agent(family, root, logs, handoff)
                    await agent.setup(environment)
                    if failure == "exception":
                        environment.cleanup_error = RuntimeError("cleanup unavailable")
                    else:
                        environment.cleanup_code = 5
                    with self.assertRaisesRegex(RuntimeError, "cleanup failed"):
                        await agent.run("work", environment, object())
                    self.assertFalse((logs / "opi-eval-terminal.json").exists())
                    self.assertEqual((logs / "stdout.log").read_bytes(), STDOUT)

    async def test_capture_failure_still_cleans_without_publishing_success(self):
        for family in INTEGRATIONS:
            root, _, logs, handoff = self.prepare(family)
            environment = FakeEnvironment()
            agent = self.agent(family, root, logs, handoff)
            await agent.setup(environment)
            environment.download_error = OSError("download failed")
            with self.assertRaisesRegex(OSError, "download failed"):
                await agent.run("work", environment, object())
            self.assertTrue(environment.cleaned)
            self.assertFalse((logs / "opi-eval-terminal.json").exists())

    async def test_existing_mounted_capture_rejects_and_cleans_private_stage(self):
        for family in INTEGRATIONS:
            for name in ("stdout.log", "stderr.log", "trace"):
                root, _, logs, handoff = self.prepare(family)
                (logs / name).write_bytes(b"prior-capture")
                environment = FakeEnvironment()
                environment.capabilities.mounted = True
                agent = self.agent(family, root, logs, handoff)
                await agent.setup(environment)
                with self.assertRaisesRegex(RuntimeError, "destination already exists"):
                    await agent.run("work", environment, object())
                self.assertTrue(environment.cleaned)
                self.assertEqual(environment.launches, [])
                self.assertEqual((logs / name).read_bytes(), b"prior-capture")
                self.assertFalse((logs / "opi-eval-terminal.json").exists())

    @unittest.skipUnless(os.name == "posix", "requires Linux symlinks")
    async def test_linked_mounted_capture_rejects_without_following_link(self):
        for family in INTEGRATIONS:
            root, _, logs, handoff = self.prepare(family)
            outside = root / "outside"
            outside.mkdir()
            logs.rmdir()
            logs.symlink_to(outside, target_is_directory=True)
            environment = FakeEnvironment()
            environment.capabilities.mounted = True
            agent = self.agent(family, root, logs, handoff)
            with self.assertRaisesRegex(RuntimeError, "unsafe runtime preflight capture"):
                await agent.setup(environment)
            self.assertTrue(environment.cleaned)
            self.assertEqual(list(outside.iterdir()), [])

    async def test_failed_stage_creation_never_uploads_and_cleans(self):
        for family in INTEGRATIONS:
            root, _, logs, handoff = self.prepare(family)
            environment = FakeEnvironment()
            environment.setup_code = 1
            with self.assertRaisesRegex(RuntimeError, "stage creation failed"):
                await self.agent(family, root, logs, handoff).setup(environment)
            self.assertEqual(environment.uploads, [])
            self.assertTrue(environment.cleaned)

    async def test_opi_headless_keyring_isolation_is_agent_scoped(self):
        for family in INTEGRATIONS:
            with self.subTest(family=family):
                root, _, logs, handoff = self.prepare(family, "opi", False)
                handoff["agent_env"]["DBUS_SESSION_BUS_ADDRESS"] = "unix:path=/unbound/session-bus"
                agent = self.agent(family, root, logs, handoff)
                environment = FakeEnvironment()
                await agent.setup(environment)
                await agent.run("work", environment, object())
                expected = {"TEST_PROJECTED": "yes", "DBUS_SESSION_BUS_ADDRESS": HEADLESS_BUS}
                self.assertEqual(environment.launches[0][1], expected)
                self.assertEqual([env for _, env in environment.exec_envs if env is not None],
                                 [environment.preflight_calls[0][1], expected])
                self.assertEqual(handoff["agent_env"]["DBUS_SESSION_BUS_ADDRESS"], "unix:path=/unbound/session-bus")
                self.assertTrue(environment.cleaned)

    async def test_pier_selected_inference_allowlist_and_proxy_are_agent_scoped(self):
        for product in ("pi", "opi"):
            with self.subTest(product=product):
                root, _, logs, handoff = self.prepare("pier", product)
                agent = self.agent("pier", root, logs, handoff)
                self.assertEqual(agent.network_allowlist().domains, ["api.selected.example"])
                environment = FakeEnvironment()
                environment.proxy_env = {
                    "HTTPS_PROXY": "http://agent:fixture@pier-egress-proxy:8080",
                    "NO_PROXY": "localhost,127.0.0.1",
                }
                await agent.setup(environment)
                self.assertEqual(environment.proxy_inputs, [])
                await agent.run("work", environment, object())
                projected = {"TEST_PROJECTED": "yes"}
                if product == "opi":
                    projected["DBUS_SESSION_BUS_ADDRESS"] = HEADLESS_BUS
                self.assertEqual(environment.proxy_inputs, [projected])
                self.assertEqual(environment.launches[0][1], {
                    **environment.proxy_env, **projected,
                })
                scoped = [env for _, env in environment.exec_envs if env is not None]
                self.assertEqual(scoped, [environment.preflight_calls[0][1], environment.launches[0][1]])
                self.assertEqual(handoff["agent_env"], {"TEST_PROJECTED": "yes"})
                self.assertTrue(environment.cleaned)

    async def test_pier_inference_endpoint_requires_exact_safe_host(self):
        for endpoint in (None, "", "http://api.selected.example/v1", "https://.selected.example",
                         "https://*.selected.example", "https://user:pass@api.selected.example",
                         "https://api.selected.example/#fragment", "https://api.selected.example/#",
                         "https://api.selected.example:8443", "https://api.selected.example:",
                         "https://api.selected.example:invalid", "https://api.selected.example\n",
                         "https://api.selected.example\\evil", "https://api.selected.example,evil",
                         "https://api..selected.example", "https://[::1]", "file:///v1",
                         "http://127.0.0.1:8080/v1"):
            with self.subTest(endpoint=endpoint):
                root, _, logs, handoff = self.prepare("pier")
                handoff["inference_endpoint"] = endpoint
                agent = self.agent("pier", root, logs, handoff)
                with self.assertRaisesRegex(RuntimeError, "inference endpoint"):
                    agent.network_allowlist()
                environment = FakeEnvironment()
                with self.assertRaisesRegex(RuntimeError, "inference endpoint"):
                    await agent.setup(environment)
                self.assertEqual(environment.uploads, [])

    async def test_pier_scripted_loopback_and_standard_https_endpoint(self):
        for endpoint, host in (("https://API.selected.example:443/v1", "api.selected.example"),
                               ("http://127.0.0.1:8080/v1", "127.0.0.1"),
                               ("http://localhost:8080/v1", "localhost")):
            with self.subTest(endpoint=endpoint):
                root, _, logs, handoff = self.prepare("pier")
                handoff.update(model="scripted:fixture", inference_endpoint=endpoint)
                agent = self.agent("pier", root, logs, handoff)
                self.assertEqual(agent.network_allowlist().domains, [host])

    @unittest.skipUnless(os.name == "posix", "requires the native Linux shell")
    async def test_real_shell_package_identity_arguments_and_artifact_match(self):
        for family in INTEGRATIONS:
            with self.subTest(family=family):
                root, runtime, logs, handoff = self.prepare(family)
                (runtime / "package.json").write_text('{"version":"0.85.1"}', encoding="utf-8")
                (runtime / "pi").write_text(
                    "#!/usr/bin/env python3\n"
                    "import json, pathlib, subprocess, sys\n"
                    "package = pathlib.Path(sys.argv[0]).parent\n"
                    "assert (package / 'theme/dark.json').read_text() == 'dark'\n"
                    "assert (package / 'theme/light.json').read_text() == 'light'\n"
                    "metadata = json.loads((package / 'package.json').read_text())\n"
                    "if sys.argv[1:] == ['--version']:\n"
                    "    print(metadata['version']); sys.exit(0)\n"
                    "subprocess.run(['/bin/sh', '-c', 'printf delivered > answer.txt'], check=True)\n"
                    "print(json.dumps({'type': 'session', 'version': 2, 'id': 'real-shell',\n"
                    "                  'timestamp': 't', 'cwd': '/work'}))\n"
                    "print(json.dumps({'type': 'tool_execution_end', 'toolName': 'bash',\n"
                    "                  'version': metadata['version'], 'argv': sys.argv[1:]}))\n"
                    "print('rate limited by provider', file=sys.stderr)\n",
                    encoding="utf-8",
                )
                handoff["agent_executable_sha256"] = hashlib.sha256((runtime / "pi").read_bytes()).hexdigest()
                canonical = "".join(
                    f"{name}\n{hashlib.sha256((runtime / name).read_bytes()).hexdigest()}\n"
                    for name in sorted(PACKAGE_FILES)
                )
                handoff["runtime_package"]["manifest_sha256"] = hashlib.sha256(canonical.encode()).hexdigest()
                environment = LocalShellEnvironment(root / "environment")
                agent = self.agent(family, root, logs, handoff)
                await agent.setup(environment)
                prompt = "--literal 'quoted'; $(touch escaped)"
                await agent.run(prompt, environment, object())
                lines = (logs / "stdout.log").read_text().splitlines()
                self.assertEqual(
                    json.loads(lines[0])["type"], "session",
                    "the sanitized stream keeps its session header",
                )
                event = json.loads(lines[1])
                self.assertEqual(event["version"], "0.85.1")
                self.assertEqual(event["toolName"], "bash")
                args = event["argv"]
                self.assertEqual(args[args.index("--provider") + 1], "zhipu")
                self.assertEqual(args[args.index("--model") + 1], "glm-5.3-flash")
                self.assertEqual(args[-2:], ["--", prompt])
                # The validated diagnostic vocabulary is retained; unknown
                # agent stderr would stay opaque and retain no byte.
                self.assertEqual(
                    (logs / "stderr.log").read_text(), "rate limited by provider\n"
                )
                self.assertFalse((environment.work / "escaped").exists())
                self.assertEqual((environment.work / "answer.txt").read_bytes(), b"delivered")
                terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                self.assertEqual(terminal["agent_executable_sha256"], handoff["agent_executable_sha256"])
                self.assertEqual(terminal["delivered"], [{
                    "source": str(environment.work / "answer.txt"),
                    "sha256": hashlib.sha256(b"delivered").hexdigest(),
                }])
                self.assertFalse(environment.local("/opi-eval-agent/pi").exists())

    @unittest.skipUnless(os.name == "posix", "requires the native Linux shell")
    async def test_partial_configuration_upload_is_removed_on_setup_failure(self):
        for family in INTEGRATIONS:
            for failure in (RuntimeError, asyncio.CancelledError):
                with self.subTest(family=family, failure=failure):
                    root, runtime, logs, handoff = self.prepare(family, package=False)
                    environment = LocalShellEnvironment(root / "environment")
                    (runtime / "pi").write_text(
                        "#!/bin/sh\nif [ \"$1\" = --version ]; then printf '0.85.1\\n'; exit 0; fi\n",
                        encoding="utf-8")
                    handoff["agent_executable_sha256"] = hashlib.sha256((runtime / "pi").read_bytes()).hexdigest()
                    environment.fail_config_upload = True
                    environment.config_upload_error = failure("partial config write")
                    agent = self.agent(family, root, logs, handoff)
                    with self.assertRaisesRegex(failure, "partial config write"):
                        await agent.setup(environment)
                    self.assertFalse(environment.local("/opi-eval-agent/pi").exists())
                    terminal = json.loads((logs / "opi-eval-terminal.json").read_text())
                    self.assertEqual(terminal["runtime_preflight"]["status"], "passed")
                    self.assertFalse(terminal["task_dispatched"])
                    self.assertIsNone(terminal["exit_code"])


if __name__ == "__main__":
    if sys.argv[1:2] == ["--capture-child"]:
        asyncio.run(capture_child(Path(sys.argv[2]), sys.argv[3], sys.argv[4],
                                  sys.argv[5] if len(sys.argv) > 5 else None))
    else:
        if "--docker" in sys.argv:
            sys.argv.remove("--docker")
        unittest.main()
