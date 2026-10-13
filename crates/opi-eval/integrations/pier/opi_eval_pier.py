"""opi-eval pinned Pier integration (P19-D04).

This module is the upstream integration opi-eval selects on the runner
argv (``--agent-import-path opi_eval_pier:OpiAgent`` / ``PiAgent``).
Pier imports it in its own process and invokes one Agent *inside the task
environment*: the selected Agent's file and shell tools operate there,
and the same upstream trial's separate pristine verifier later grades the
committed patch the task's collect hook extracts. An independent
host-side Agent run followed by an unrelated job is not this contract.

The integration is driven exclusively by the ``OPI_EVAL_NATIVE_HANDOFF``
environment variable the opi-eval runner exports for the dispatched job
(schema ``opi-eval-native-handoff/4``): the exact built Agent executable,
the projected selected connection, and the credential material. None of
it is read from ambient user configuration, and the integration never
improvises or falls back to another provider, protocol, or model.
Credentials are applied only to the Agent process inside the task
environment and removed before the verifier phase runs.

Before projecting credentials, the integration checks the staged runtime with
a bounded, supervised `--version` command. Separate captures bind the
compatibility proof; failure or uncertainty prevents task dispatch.

After refusal or the Agent phase, the integration writes a /2 terminal observation
(``opi-eval-terminal.json`` next to its Agent logs): the observed exit
code, whether the Agent hit its timeout, the sha256 of the exact Agent
executable it executed, and the bounded manifest of files the Agent
delivered in the environment working directory. The opi-eval result
reader binds the imported job against this observation.

The module uses only the pinned Pier v0.3.1 ``BaseAgent`` surface
(setup/run, async environment exec/upload) and the Python standard
library.
"""

from __future__ import annotations

import asyncio
import base64
import hashlib
import json
import os
import re
import shlex
import stat
import time
import types
import uuid
from pathlib import Path, PurePosixPath
from typing import Any
from urllib.parse import urlsplit

from pier.agents.base import BaseAgent
from pier.environments.base import BaseEnvironment
from pier.environments.docker.docker import DockerEnvironment
from pier.models.agent.context import AgentContext
from pier.models.agent.network import NetworkAllowlist

_HANDOFF_ENV = "OPI_EVAL_NATIVE_HANDOFF"
_HANDOFF_SCHEMA = "opi-eval-native-handoff/4"
_TERMINAL_SCHEMA = "opi-eval-native-agent-terminal/3"
_TERMINAL_FILENAME = "opi-eval-terminal.json"
_PREFLIGHT_CAPTURE_BYTES = 64 * 1024 * 1024

# Where the exact built Agent executable and its projected configuration
# are staged inside the task environment. The directory is removed again
# before the verifier phase (credential cleanup): the pristine verifier
# receives neither provider credentials nor unrelated host files.
_ENV_STAGE_DIR = PurePosixPath("/opi-eval-agent")

# Bounded delivered-artifact observation: at most this many regular files
# under the environment working directory, each at most this many bytes.
_MAX_DELIVERED_FILES = 256
_MAX_DELIVERED_BYTES = 64 * 1024 * 1024


class _OpiEvalAgentError(RuntimeError):
    """Raised for integration contract violations (fail-closed)."""


class _RuntimePreflightOutputError(RuntimeError):
    """The supervised version command ended without readable capture proof."""


class _NativeCommandTerminationError(RuntimeError):
    """Owned supervision could not observe the container-command termination."""



class OwnedDockerEnvironment(DockerEnvironment):
    """Supervise each native exec through its observed container-command exit."""

    async def _settle_shielded(self, task, *, propagate_cancellation=False):
        return await _settle_shielded_task(task, propagate_cancellation=propagate_cancellation)

    def _effect_unknown(self, reason):
        (self.trial_paths.trial_dir / "opi-eval-effect-unknown").write_text(
            reason + "\n", encoding="utf-8",
        )

    async def exec(self, command, cwd=None, env=None, timeout_sec=None, user=None,
                   *, _capture_limit=None):
        identity = uuid.uuid4().hex
        state = f"/tmp/opi-eval-exec-{identity}"
        # Bash job control creates one process group for this invocation.
        # Keep the transport alive until wait reports that group's actual exit.
        wrapped = (
            f"(umask 077; mkdir {state}) || exit 125\n"
            "set -m\n"
            f"bash -c {shlex.quote(command)} &\n"
            "opi_eval_pid=$!\n"
            f"printf '%s\\n' \"$opi_eval_pid\" > {state}/pid\n"
            "wait \"$opi_eval_pid\"\n"
            "opi_eval_code=$?\n"
            f"rm -f {state}/pid; rmdir {state}\n"
            "exit \"$opi_eval_code\""
        )
        marker = f"opi-eval-runtime-capture-{identity}"
        if _capture_limit is not None:
            # Bound the streams before the upstream Docker transport merges
            # stderr. The extra byte proves overflow without retaining an
            # unbounded stream; only this invocation owns these private files.
            wrapped = (
                f"(umask 077; mkdir {state}) || exit 125\n"
                "set -m\n"
                f"exec 3> >(head -c {_capture_limit + 1} > {state}/stdout)\n"
                "opi_eval_stdout_pid=$!\n"
                f"exec 4> >(head -c {_capture_limit + 1} 3>&- > {state}/stderr)\n"
                "opi_eval_stderr_pid=$!\n"
                f"bash -c {shlex.quote(command)} >&3 2>&4 &\n"
                "opi_eval_pid=$!\n"
                f"printf '%s\\n' \"$opi_eval_pid\" > {state}/pid\n"
                "exec 3>&- 4>&-\n"
                "wait \"$opi_eval_pid\"\n"
                "opi_eval_code=$?\n"
                "wait \"$opi_eval_stdout_pid\" \"$opi_eval_stderr_pid\"\n"
                f"printf '\\n{marker}\\n'\n"
                f"base64 -w 0 {state}/stdout; printf '\\n'\n"
                f"base64 -w 0 {state}/stderr; printf '\\n'\n"
                f"rm -f {state}/pid {state}/stdout {state}/stderr; rmdir {state}\n"
                "exit \"$opi_eval_code\""
            )
        started = time.monotonic()
        launch = asyncio.create_task(super().exec(
            wrapped, cwd=cwd, env=env, timeout_sec=None, user=user,
        ))
        termination = "completed"
        result = None
        try:
            if timeout_sec is None:
                result = await asyncio.shield(launch)
            else:
                result = await asyncio.wait_for(asyncio.shield(launch), timeout_sec)
            return self._runtime_capture_result(result, marker) if _capture_limit is not None else result
        except (asyncio.CancelledError, TimeoutError) as cause:
            termination = "cancelled" if isinstance(cause, asyncio.CancelledError) else "timeout"

            async def settle():
                try:
                    async with asyncio.timeout(5):
                        if not launch.done():
                            stop = (
                                f"for i in {{1..100}}; do test -f {state}/pid && break; sleep 0.01; done\n"
                                f"if read -r pid < {state}/pid; then\n"
                                "[[ $pid =~ ^[1-9][0-9]*$ ]] && kill -KILL -- -\"$pid\"\n"
                                "fi"
                            )
                            await super(OwnedDockerEnvironment, self).exec(
                                stop, user="root", timeout_sec=2,
                            )
                        return await asyncio.shield(launch)
                except Exception as exc:
                    self._effect_unknown("native command exit could not be observed")
                    # This uses the upstream environment's exact compose project.
                    # No shared image, neighboring project, or global prune is used.
                    try:
                        await self._run_docker_compose_command(["stop"], timeout_sec=5)
                        await asyncio.wait_for(asyncio.shield(launch), 5)
                    except Exception:
                        pass
                    raise _NativeCommandTerminationError("native command termination is uncertain") from exc

            result = await self._settle_shielded(asyncio.create_task(settle()))
            if _capture_limit is not None:
                try:
                    cause.runtime_capture = self._runtime_capture_result(result, marker)
                except _RuntimePreflightOutputError:
                    pass
            raise
        except _RuntimePreflightOutputError:
            raise
        except Exception:
            self._effect_unknown("native exec transport failed")
            raise
        finally:
            record = {
                "command_id": identity,
                "command_sha256": hashlib.sha256(command.encode()).hexdigest(),
                "elapsed_seconds": time.monotonic() - started,
                "exit_code": None if result is None else result.return_code,
                "termination": termination,
            }
            with (self.trial_paths.trial_dir / "opi-eval-commands.jsonl").open(
                "a", encoding="utf-8",
            ) as log:
                log.write(json.dumps(record, sort_keys=True) + "\n")

    @staticmethod
    def _runtime_capture_result(result, marker):
        output = result.stdout
        if not isinstance(output, str):
            raise _RuntimePreflightOutputError("runtime preflight capture is unreadable")
        _, found, encoded = output.rpartition(marker + "\n")
        rows = encoded.split("\n")
        if not found or len(rows) != 3 or rows[-1] != "":
            raise _RuntimePreflightOutputError("runtime preflight capture is incomplete")
        try:
            stdout, stderr = (base64.b64decode(row, validate=True) for row in rows[:2])
        except (ValueError, UnicodeError) as exc:
            raise _RuntimePreflightOutputError("runtime preflight capture is invalid") from exc
        return types.SimpleNamespace(return_code=result.return_code, stdout=stdout, stderr=stderr)

    async def _run_docker_compose_command(self, command, **kwargs):
        command = list(command)
        if command and command[0] == "down" and "--rmi" in command:
            index = command.index("--rmi")
            del command[index:index + 2]
        # A transient compose failure is not itself cleanup uncertainty:
        # upstream teardown tolerates and retries some failures, and the
        # stop() settle pass confirms the final state by observing residue.
        return await super()._run_docker_compose_command(command, **kwargs)

    async def stop(self, delete):
        async def settle():
            try:
                # A CPU-saturated task container can need tens of seconds for
                # a teardown; the bound stays finite so cleanup uncertainty
                # still fails closed.
                async with asyncio.timeout(120):
                    try:
                        await super(OwnedDockerEnvironment, self).stop(delete)
                    except Exception:
                        # Upstream teardown logs and retries some compose
                        # failures without raising them; the residue check
                        # below decides whether cleanup is actually confirmed.
                        pass
                    remaining = await self._run_docker_compose_command(
                        ["ps", "--all", "--quiet"], timeout_sec=5,
                    )
                    if remaining.stdout and remaining.stdout.strip():
                        raise RuntimeError("owned environment resources remain")
                    marker = self.trial_paths.trial_dir / "opi-eval-effect-unknown"
                    if marker.is_file() and marker.read_text(encoding="utf-8").strip() == "owned environment cleanup failed":
                        # A previous stop() attempt marked this trial before a
                        # transient failure; the residue check has now
                        # confirmed the environment is gone, so that recorded
                        # cleanup uncertainty is resolved. Uncertainty markers
                        # from other phases keep their meaning and stay.
                        marker.unlink()
            except Exception:
                self._effect_unknown("owned environment cleanup failed")
                raise
        await self._settle_shielded(asyncio.create_task(settle()), propagate_cancellation=True)

async def _settle_shielded_task(task, *, propagate_cancellation=False):
    """Await a bounded settlement to completion even under cancellation."""
    cancelled = False
    while True:
        try:
            result = await asyncio.shield(task)
            break
        except asyncio.CancelledError:
            cancelled = True
            if task.done():
                result = task.result()
                break
    if cancelled and propagate_cancellation:
        raise asyncio.CancelledError
    return result


class _OpiEvalAgent(BaseAgent):
    """Shared implementation for the opi-eval selected Agents."""

    _PRODUCT = "<override>"

    def __init__(
        self,
        logs_dir: Path,
        model_name: str | None = None,
        logger: Any = None,
        **kwargs,
    ):
        super().__init__(
            logs_dir=logs_dir,
            model_name=model_name,
            logger=logger,
            **kwargs,
        )
        self._handoff_path, self._handoff = self._load_handoff()
        self._timed_out = False
        self._runtime_entrypoint: PurePosixPath | None = None
        self._executable_sha256: str | None = None
        self._capture_root = self._stage_dir()
        self._capture_mounted = False
        self._runtime_preflight: dict[str, Any] | None = None
        self._helper_preflight: dict[str, Any] | None = None
        self._capture_helper: dict[str, Any] | None = None
        self._capture_policy: str | None = None
        self._task_dispatched = False

    def _load_handoff(self) -> tuple[Path, dict[str, Any]]:
        path = os.environ.get(_HANDOFF_ENV)
        if not path:
            raise _OpiEvalAgentError(
                "opi-eval integration requires OPI_EVAL_NATIVE_HANDOFF"
            )
        handoff_path = Path(path)
        handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
        if handoff.get("schema") != _HANDOFF_SCHEMA:
            raise _OpiEvalAgentError("unknown opi-eval handoff schema")
        if handoff.get("product") != self._PRODUCT:
            raise _OpiEvalAgentError(
                f"handoff product {handoff.get('product')!r} does not select "
                f"{self.name()}"
            )
        name = handoff.get("integration_agent_name")
        if name != self.name():
            raise _OpiEvalAgentError(
                f"handoff integration identity {name!r} does not select "
                f"{self.name()}"
            )
        trial = handoff.get("trial")
        if (not isinstance(trial, str) or not trial
                or any(not (char.isascii() and (char.isalnum() or char in "-_.")) for char in trial)
                or ".." in trial):
            raise _OpiEvalAgentError("handoff trial identity is missing or unsafe")
        return handoff_path, handoff

    @staticmethod
    def _digest_file(path: Path) -> str:
        digest = hashlib.sha256()
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        return digest.hexdigest()

    def _stage_dir(self) -> PurePosixPath:
        return _ENV_STAGE_DIR / self._PRODUCT

    @staticmethod
    def _safe_relative(value: str) -> PurePosixPath:
        if (
            not isinstance(value, str)
            or not value
            or any(ord(char) < 32 for char in value)
            or "\\" in value
            or ":" in value
        ):
            raise _OpiEvalAgentError("unsafe runtime package path")
        path = PurePosixPath(value)
        if path.is_absolute() or ".." in path.parts or path.as_posix() != value or value == ".":
            raise _OpiEvalAgentError("unsafe runtime package path")
        return path

    def _verify_runtime(self) -> Path | None:
        executable = Path(self._handoff["agent_executable"])
        if (
            not executable.is_absolute()
            or any(path.is_symlink() for path in (executable, *executable.parents))
            or not executable.is_file()
        ):
            raise _OpiEvalAgentError("handoff agent executable is missing or unsafe")
        self._executable_sha256 = self._digest_file(executable)
        if self._executable_sha256 != self._handoff.get("agent_executable_sha256"):
            raise _OpiEvalAgentError("agent executable digest mismatch")

        helper = self._handoff.get("capture_helper")
        if (
            not isinstance(helper, dict)
            or set(helper) != {"path", "sha256", "target", "protocol"}
        ):
            raise _OpiEvalAgentError("handoff capture helper identity is missing")
        helper_path = Path(helper["path"])
        if (
            not helper_path.is_absolute()
            or any(path.is_symlink() for path in (helper_path, *helper_path.parents))
            or not helper_path.is_file()
        ):
            raise _OpiEvalAgentError("handoff capture helper is missing or unsafe")
        if self._digest_file(helper_path) != helper["sha256"]:
            raise _OpiEvalAgentError("capture helper digest mismatch")
        if helper["protocol"] != "opi-eval-native-capture-helper/1":
            raise _OpiEvalAgentError("capture helper protocol mismatch")
        self._capture_helper = helper
        self._capture_policy = self._handoff.get("capture_policy")
        if self._capture_policy not in ("full", "terminal-only"):
            raise _OpiEvalAgentError("handoff capture policy is missing")

        package = self._handoff.get("runtime_package")
        if package is None:
            return None
        if not isinstance(package, dict) or set(package) != {"path", "manifest_sha256", "entrypoint"}:
            raise _OpiEvalAgentError("invalid runtime package")
        root = Path(package["path"])
        if (
            not root.is_absolute()
            or ".." in root.parts
            or any(path.is_symlink() for path in (root, *root.parents))
            or not root.is_dir()
        ):
            raise _OpiEvalAgentError("unsafe runtime package directory")
        entrypoint = self._safe_relative(package["entrypoint"])
        rows = []
        pending = [root]
        while pending:
            for path in pending.pop().iterdir():
                relative = self._safe_relative(path.relative_to(root).as_posix())
                mode = path.lstat().st_mode
                if stat.S_ISDIR(mode):
                    pending.append(path)
                elif stat.S_ISREG(mode):
                    rows.append((relative.as_posix(), self._digest_file(path)))
                else:
                    raise _OpiEvalAgentError("runtime package contains a symlink or special file")
        canonical = "".join(f"{path}\n{digest}\n" for path, digest in sorted(rows))
        digest = hashlib.sha256(canonical.encode("utf-8")).hexdigest()
        if digest != package["manifest_sha256"]:
            raise _OpiEvalAgentError("runtime package manifest digest mismatch")
        if dict(rows).get(entrypoint.as_posix()) != self._executable_sha256:
            raise _OpiEvalAgentError("runtime package entrypoint digest mismatch")
        self._runtime_entrypoint = entrypoint
        return root

    def _staged_executable(self, stage: PurePosixPath) -> str:
        path = (
            stage / "runtime" / self._runtime_entrypoint
            if self._runtime_entrypoint else stage / "agent"
        )
        return shlex.quote(str(path))

    def network_allowlist(self) -> NetworkAllowlist:
        # Pier applies this only to the Agent environment; its pristine
        # verifier receives no Agent allowlist or proxy credentials.
        endpoint = self._handoff.get("inference_endpoint")
        try:
            if (
                not isinstance(endpoint, str)
                or any(char.isspace() or ord(char) < 32 for char in endpoint)
                or "\\" in endpoint
                or "#" in endpoint
            ):
                raise ValueError
            parsed = urlsplit(endpoint)
            host = (parsed.hostname or "").rstrip(".")
            if (
                not host
                or parsed.username is not None
                or parsed.password is not None
                or parsed.netloc.endswith(":")
                or any(not re.fullmatch(r"[a-z0-9](?:[a-z0-9-]*[a-z0-9])?", label)
                       for label in host.split("."))
            ):
                raise ValueError
            # The pinned proxy supports HTTPS CONNECT on 443. Scripted
            # loopback HTTP stays local through its existing NO_PROXY rule.
            scripted_loopback = (
                str(self._handoff.get("model", "")).partition(":")[0] == "scripted"
                and host in ("localhost", "127.0.0.1")
                and parsed.scheme == "http"
                and parsed.port != 0
            )
            if not scripted_loopback and not (
                parsed.scheme == "https" and parsed.port in (None, 443)
            ):
                raise ValueError
            return NetworkAllowlist(domains=[host])
        except ValueError as exc:
            raise _OpiEvalAgentError("invalid handoff inference endpoint") from exc

    async def setup(self, environment: BaseEnvironment) -> None:
        # Verify the entire runtime before any task-environment upload.
        self.network_allowlist()
        stage = self._stage_dir()
        package_root = self._verify_runtime()
        model = self._handoff.get("model")
        if (
            not isinstance(model, str)
            or ":" not in model
            or any(not part or part.strip() != part for part in model.split(":", 1))
        ):
            raise _OpiEvalAgentError("handoff model must be exact provider:model")
        config_name = "models.json" if self._PRODUCT == "pi" else "config.toml"
        config_path = self._handoff_path.parent / config_name
        if (
            self._handoff.get("config_path", config_name) != config_name
            or config_path.is_symlink()
            or not config_path.is_file()
        ):
            raise _OpiEvalAgentError("handoff configuration is missing or unsafe")
        try:
            result = await environment.exec(
                command=f"mkdir -p {shlex.quote(str(stage))} && chmod 700 {shlex.quote(str(stage))}",
                user="root",
            )
            if result.return_code != 0:
                raise _OpiEvalAgentError("agent stage creation failed")
            if package_root is None:
                await environment.upload_file(Path(self._handoff["agent_executable"]), f"{stage}/agent")
            else:
                await environment.upload_dir(package_root, f"{stage}/runtime")
            result = await environment.exec(
                command=f"chmod 700 {self._staged_executable(stage)}",
                user="root",
            )
            if result.return_code != 0:
                raise _OpiEvalAgentError("agent executable staging failed")
            # The shared capture helper is staged beside the Agent before
            # any credential projection and proves itself inside the task
            # environment; a failed helper preflight prevents dispatch.
            await environment.upload_file(
                Path(self._capture_helper["path"]), f"{stage}/capture-helper"
            )
            result = await environment.exec(
                command=f"chmod 700 {shlex.quote(str(stage))}/capture-helper",
                user="root",
            )
            if result.return_code != 0:
                raise _OpiEvalAgentError("capture helper staging failed")
            await self._runtime_preflight_check(environment, stage)
            await self._helper_preflight_check(environment, stage)
            await self._project_configuration(environment, stage)
            # Configuration upload owns fixed sibling paths, never runtime
            # files. Recheck source identities before allowing task dispatch.
            self._verify_runtime()
        except BaseException:  # Cleanup also applies when the runner cancels setup.
            try:
                if (self._runtime_preflight is not None
                        and not any(path.is_symlink() for path in (self.logs_dir, *self.logs_dir.parents))
                        and not (self.logs_dir / _TERMINAL_FILENAME).exists()):
                    self._write_terminal_observation(environment, None, [])
            finally:
                await self._cleanup(environment, stage)
            raise

    async def _project_configuration(
        self, environment: BaseEnvironment, stage: PurePosixPath
    ) -> None:
        raise NotImplementedError

    async def _runtime_preflight_check(self, environment, stage):
        command = self._staged_executable(stage) + " --version"
        home = f"{stage}/runtime-preflight-home"
        env = {
            "HOME": home, "XDG_CONFIG_HOME": f"{home}/config",
            "XDG_DATA_HOME": f"{home}/data", "XDG_CACHE_HOME": f"{home}/cache",
            "PI_CODING_AGENT_DIR": home,
        }
        if self._PRODUCT == "opi":
            env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:abstract={stage}/no-keyring"
        package = self._handoff.get("runtime_package")
        proof = {
            "kind": "version", "status": "unknown", "exit_code": None,
            "agent_executable_sha256": self._executable_sha256,
            "runtime_package_manifest_sha256": package["manifest_sha256"] if package else None,
            "runtime_package_entrypoint": package["entrypoint"] if package else None,
            "command_sha256": hashlib.sha256(command.encode()).hexdigest(),
            "stdout_sha256": hashlib.sha256(b"").hexdigest(),
            "stderr_sha256": hashlib.sha256(b"").hexdigest(),
            "stdout_complete": False, "stderr_complete": False,
            "stdout_truncated": False, "stderr_truncated": False,
            "reason": None,
        }
        result = None
        interrupted = None
        try:
            kwargs = {"command": command, "env": env, "timeout_sec": 30}
            if isinstance(environment, OwnedDockerEnvironment):
                kwargs["_capture_limit"] = _PREFLIGHT_CAPTURE_BYTES
            # The probe is idempotent and side-effect free; a docker-exec
            # transport flake (exit 0 with an empty marker frame) gets one
            # immediate retry before it can fail the whole run at the
            # integrity boundary.
            for attempt in (1, 2, 3):
                try:
                    result = await environment.exec(**kwargs)
                    break
                except _RuntimePreflightOutputError:
                    if attempt == 3:
                        raise
                    self.logger.warning(
                        "runtime preflight capture empty; retrying (attempt %d)", attempt)
                    await asyncio.sleep(3)
        except BaseException as exc:
            interrupted = exc
            result = getattr(exc, "runtime_capture", None)
            proof["reason"] = (
                "timeout" if isinstance(exc, TimeoutError)
                else "termination" if isinstance(exc, (asyncio.CancelledError, _NativeCommandTerminationError))
                else "invalid-output" if isinstance(exc, _RuntimePreflightOutputError)
                else "transport"
            )
        captures = {}
        for stream in ("stdout", "stderr"):
            raw = getattr(result, stream, None)
            try:
                if isinstance(raw, str):
                    raw = raw.encode("utf-8", errors="strict")
                if not isinstance(raw, bytes):
                    raise ValueError("missing stream")
                overflow = len(raw) > _PREFLIGHT_CAPTURE_BYTES
                raw = raw[:_PREFLIGHT_CAPTURE_BYTES + 1]
                bounded = raw[:_PREFLIGHT_CAPTURE_BYTES]
                proof[f"{stream}_sha256"] = hashlib.sha256(bounded).hexdigest()
                proof[f"{stream}_truncated"] = overflow
                if overflow:
                    proof["reason"] = proof["reason"] or "capture-overflow"
                bounded.decode("utf-8", errors="strict")
                if b"\0" in bounded:
                    raise ValueError("invalid stream")
                proof[f"{stream}_complete"] = interrupted is None and not overflow
            except (ValueError, UnicodeError):
                raw = raw if isinstance(raw, bytes) else b""
                raw = raw[:_PREFLIGHT_CAPTURE_BYTES + 1]
                proof[f"{stream}_sha256"] = hashlib.sha256(raw[:_PREFLIGHT_CAPTURE_BYTES]).hexdigest()
                proof["reason"] = proof["reason"] or "invalid-output"
            captures[stream] = raw
        code = getattr(result, "return_code", None)
        if type(code) is int and -(2**31) <= code < 2**31:
            proof["exit_code"] = code
        elif interrupted is None:
            proof["reason"] = proof["reason"] or "invalid-output"
        complete = proof["stdout_complete"] and proof["stderr_complete"]
        if complete and proof["exit_code"] is not None and proof["reason"] is None:
            if code != 0:
                proof["status"] = "failed"
            elif (captures["stdout"] + captures["stderr"]).strip():
                proof["status"] = "passed"
            else:
                proof["reason"] = "invalid-output"
        self._runtime_preflight = proof
        if any(path.is_symlink() for path in (self.logs_dir, *self.logs_dir.parents)):
            raise _OpiEvalAgentError("unsafe runtime preflight capture directory")
        self.logs_dir.mkdir(parents=True, exist_ok=True)
        for stream, raw in captures.items():
            path = self.logs_dir / f"runtime-preflight-{stream}.log"
            flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
            with os.fdopen(os.open(path, flags, 0o600), "wb") as output:
                output.write(raw)
        if proof["status"] != "passed":
            self._write_terminal_observation(environment, None, [])
            if interrupted is not None:
                raise interrupted
            raise _OpiEvalAgentError(f"runtime preflight {proof['status']}")

    async def _helper_preflight_check(self, environment, stage):
        command = f"{shlex.quote(str(stage))}/capture-helper __native-capture --selfcheck"
        proof = {
            "kind": "capture-helper-selfcheck", "status": "unknown", "exit_code": None,
            "command_sha256": hashlib.sha256(command.encode()).hexdigest(),
            "stdout_sha256": hashlib.sha256(b"").hexdigest(),
            "stderr_sha256": hashlib.sha256(b"").hexdigest(),
            "stdout_complete": False, "stderr_complete": False,
            "stdout_truncated": False, "stderr_truncated": False, "reason": None,
        }
        result = None
        interrupted = None
        try:
            kwargs = {"command": command, "timeout_sec": 30}
            if isinstance(environment, OwnedDockerEnvironment):
                kwargs["_capture_limit"] = _PREFLIGHT_CAPTURE_BYTES
            # The probe is idempotent and side-effect free; a docker-exec
            # transport flake (exit 0 with an empty marker frame) gets one
            # immediate retry before it can fail the whole run at the
            # integrity boundary.
            for attempt in (1, 2, 3):
                try:
                    result = await environment.exec(**kwargs)
                    break
                except _RuntimePreflightOutputError:
                    if attempt == 3:
                        raise
                    self.logger.warning(
                        "runtime preflight capture empty; retrying (attempt %d)", attempt)
                    await asyncio.sleep(3)
        except BaseException as exc:
            interrupted = exc
            proof["reason"] = (
                "timeout" if isinstance(exc, TimeoutError)
                else "termination" if isinstance(exc, (asyncio.CancelledError, _NativeCommandTerminationError))
                else "transport"
            )
        identity = None
        raw = getattr(result, "stdout", None)
        try:
            if isinstance(raw, str):
                raw = raw.encode("utf-8", errors="strict")
            if not isinstance(raw, bytes):
                raise ValueError("missing stream")
            bounded = raw[:_PREFLIGHT_CAPTURE_BYTES]
            proof["stdout_sha256"] = hashlib.sha256(bounded).hexdigest()
            proof["stdout_complete"] = interrupted is None and len(raw) <= _PREFLIGHT_CAPTURE_BYTES
            proof["stdout_truncated"] = len(raw) > _PREFLIGHT_CAPTURE_BYTES
            identity = json.loads(bounded.decode("utf-8", errors="strict"))
        except (ValueError, UnicodeError):
            proof["reason"] = proof["reason"] or "invalid-output"
        stderr_raw = getattr(result, "stderr", None)
        try:
            if isinstance(stderr_raw, str):
                stderr_raw = stderr_raw.encode("utf-8", errors="strict")
            if not isinstance(stderr_raw, bytes):
                raise ValueError("missing stream")
            bounded = stderr_raw[:_PREFLIGHT_CAPTURE_BYTES]
            proof["stderr_sha256"] = hashlib.sha256(bounded).hexdigest()
            proof["stderr_complete"] = interrupted is None and len(stderr_raw) <= _PREFLIGHT_CAPTURE_BYTES
            proof["stderr_truncated"] = len(stderr_raw) > _PREFLIGHT_CAPTURE_BYTES
        except (ValueError, UnicodeError):
            proof["reason"] = proof["reason"] or "invalid-output"
        code = getattr(result, "return_code", None)
        if type(code) is int and -(2**31) <= code < 2**31:
            proof["exit_code"] = code
        if (
            proof["stdout_complete"] and proof["stderr_complete"]
            and proof["exit_code"] == 0 and proof["reason"] is None
            and isinstance(identity, dict)
            and identity.get("schema") == "opi-eval-native-capture-helper/1"
            and identity.get("protocol") == "opi-eval-native-capture/1"
        ):
            proof["status"] = "passed"
        elif proof["reason"] is None and interrupted is None:
            proof["status"] = "failed"
        self._helper_preflight = proof
        if proof["status"] != "passed":
            self._write_terminal_observation(environment, None, [])
            if interrupted is not None:
                raise interrupted
            raise _OpiEvalAgentError(f"capture helper preflight {proof['status']}")

    def _agent_command(self, instruction: str, stage: PurePosixPath) -> str:
        raise NotImplementedError

    def _agent_env(self) -> dict[str, str]:
        # The projected agent environment from the handoff only: never
        # ambient user values.
        return dict(self._handoff.get("agent_env") or {})

    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        stage = self._stage_dir()
        # Both pinned Docker runners bind only this trial's Agent log directory.
        # Keep configuration/runtime staging private; a killed host runner cannot
        # download from that staging directory before owned-container removal.
        self._capture_mounted = environment.capabilities.mounted
        try:
            self._verify_runtime()
            if self._capture_mounted:
                if any(path.is_symlink() for path in (self.logs_dir, *self.logs_dir.parents)):
                    raise _OpiEvalAgentError("unsafe mounted capture directory")
                self.logs_dir.mkdir(parents=True, exist_ok=True)
                if any((self.logs_dir / name).exists() or (self.logs_dir / name).is_symlink()
                       for name in ("stdout.log", "stderr.log", "trace", "capture-receipt.json")):
                    raise _OpiEvalAgentError("mounted capture destination already exists")
                self._capture_root = PurePosixPath("/logs/agent")
        except BaseException:
            await self._cleanup(environment, stage)
            raise
        exit_code = None
        cancelled = False
        launch_error = None
        # The pinned Docker exec uses Bash. Job control gives this launch its
        # own process group; the private pid file lives only until cleanup.
        # First-sink privacy boundary (P19-AUD-003): the capture helper
        # spawns the Agent through bounded pipes and owns stdout.log,
        # stderr.log and the capture receipt; raw shell redirects are
        # gone. The helper leads the job's process group, so the owned
        # cancellation and the observed exit keep their meaning.
        helper = f"{shlex.quote(str(stage))}/capture-helper"
        command = (
            "set -m\n"
            f"{helper} __native-capture"
            f" --policy {self._capture_policy}"
            f" --stdout {shlex.quote(f'{self._capture_root}/stdout.log')}"
            f" --stderr {shlex.quote(f'{self._capture_root}/stderr.log')}"
            f" --receipt {shlex.quote(f'{self._capture_root}/capture-receipt.json')}"
            f" -- {self._agent_command(instruction, stage)} &\n"
            "opi_eval_pid=$!\n"
            f"printf '%s\\n' \"$opi_eval_pid\" > {shlex.quote(f'{stage}/agent.pid')}\n"
            "wait \"$opi_eval_pid\""
        )
        if self._runtime_preflight is None or self._runtime_preflight["status"] != "passed":
            await self._cleanup(environment, stage)
            raise _OpiEvalAgentError("task dispatch requires passed runtime preflight")
        if self._helper_preflight is None or self._helper_preflight["status"] != "passed":
            await self._cleanup(environment, stage)
            raise _OpiEvalAgentError("task dispatch requires passed capture helper preflight")
        self._task_dispatched = True
        launch = asyncio.create_task(environment.exec(
            command=f"bash -c {shlex.quote(command)}",
            env=environment.agent_process_env(self._agent_env()),
        ))
        try:
            result = await asyncio.shield(launch)
            exit_code = int(result.return_code)
        except asyncio.CancelledError:
            cancelled = True
            if not launch.cancelled():
                # Keep the transport alive to observe the actual exit. Killing
                # its client alone can leave the Agent running in the container.
                # A CPU-saturated (compute-saturated) container can need tens of seconds even for
                # the pid-file read and group kill; the bound stays finite.
                async with asyncio.timeout(60):
                    if not launch.done():
                        pid_file = shlex.quote(f"{stage}/agent.pid")
                        stop = (
                            f"for i in {{1..100}}; do test -f {pid_file} && break; sleep 0.01; done; "
                            f"read -r pid < {pid_file}; "
                            "[[ $pid =~ ^[1-9][0-9]*$ ]] && "
                            "kill -KILL -- -\"$pid\""
                        )
                        stopped = await environment.exec(
                            command=f"bash -c {shlex.quote(stop)}", user="root",
                        )
                        if stopped.return_code != 0:
                            raise _OpiEvalAgentError("Agent process-group termination failed")
                    result = await asyncio.shield(launch)
                    exit_code = int(result.return_code)
                    cancelled = False
            # Only the runner distinguishes a stage deadline from other
            # cancellation. Preserve that signal; do not invent a timeout.
            raise
        except Exception as exc:  # noqa: BLE001 - settle every terminal shape
            name = type(exc).__name__
            if isinstance(exc, TimeoutError):
                self._timed_out = True
            launch_error = exc
            self.logger.warning("agent phase ended with %s", name)
        finally:
            # Settlement is shielded: repeated caller cancellation (the agent
            # timeout cancels the whole run) must not interrupt it, and the
            # terminal observation is written inside — after cleanup succeeds
            # (a failed cleanup never publishes a settlement) but even when
            # the capture awaits were cancelled. Without the observation the
            # native binding aborts the whole run at the integrity boundary
            # instead of settling one honest invalid.
            async def settle_effects():
                delivered = []
                try:
                    await self._capture_logs(environment, stage)
                    delivered = await self._observe_delivered(environment)
                finally:
                    await self._cleanup(environment, stage)
                # Always published, including cancellation (harbor converts
                # its agent-timeout cancellation into AgentTimeoutError): an
                # honest terminal with an unobserved exit settles one invalid
                # trial, while a missing observation aborts the whole run.
                self._write_terminal_observation(environment, exit_code, delivered)
            await _settle_shielded_task(asyncio.ensure_future(settle_effects()), propagate_cancellation=True)
        if launch_error is not None:
            raise launch_error
        if exit_code not in (0,):
            # The Agent's own non-zero exit or timeout is a graded Agent
            # outcome: the verifier still grades the collected patch.
            # Raise only the pinned runner's classified error so the
            # trial records it (NonZeroAgentExitCodeError family).
            from pier.agents.installed.base import NonZeroAgentExitCodeError

            raise NonZeroAgentExitCodeError(
                f"opi-eval agent exited with {exit_code}"
                + (" (timeout)" if self._timed_out else "")
            )

    async def _capture_logs(self, environment: BaseEnvironment, stage: PurePosixPath) -> None:
        if self._capture_mounted:
            # The live bind mount exposes the helper's capture files in the
            # trial logs directory in real time, but they stay owned by the
            # container user with mode 0600: this host process cannot read
            # them until the runner's post-stop chown, long after the
            # terminal observation. Copy just the receipt through the
            # daemon (readable, owned by this user) so the observation
            # reads actual capture bytes; streams and the trace directory
            # stay runner-owned, because a pre-populated trace destination
            # breaks the runner's later compose collection.
            self.logs_dir.mkdir(parents=True, exist_ok=True)
            try:
                await environment.download_file(
                    f"{self._capture_root}/capture-receipt.json",
                    self.logs_dir / "capture-receipt.json",
                )
            except Exception:  # noqa: BLE001 - a dead helper leaves no receipt
                # A cancelled or killed helper may not have written its
                # receipt; the terminal observation then honestly records
                # an unreadable capture instead of failing settlement.
                self.logger.warning("capture receipt copy failed", exc_info=True)
            return
        self.logs_dir.mkdir(parents=True, exist_ok=True)
        for name in ("stdout.log", "stderr.log", "capture-receipt.json"):
            await environment.download_file(f"{stage}/{name}", self.logs_dir / name)
        if self._PRODUCT == "opi" and await environment.is_dir(f"{stage}/trace", user="root"):
            await environment.download_dir(f"{stage}/trace", self.logs_dir / "trace")

    async def _cleanup(self, environment: BaseEnvironment, stage: PurePosixPath) -> None:
        # A failed cleanup must prevent successful settlement and grading.
        try:
            result = await environment.exec(
                command=f"rm -rf {shlex.quote(str(stage))} && test ! -e {shlex.quote(str(stage))}",
                user="root",
            )
            if result.return_code != 0:
                raise _OpiEvalAgentError("agent stage cleanup failed")
        except Exception as exc:
            raise _OpiEvalAgentError("agent stage cleanup failed") from exc

    async def _observe_delivered(
        self, environment: BaseEnvironment
    ) -> list[dict[str, Any]]:
        """Bounded manifest of files in the environment working directory."""
        try:
            listing = await environment.exec(
                command=(
                    "pwd && find . -maxdepth 3 -type f "
                    f"-size -{_MAX_DELIVERED_BYTES}c -print0 "
                    "| xargs -0 sha256sum | head -n "
                    f"{_MAX_DELIVERED_FILES}"
                ),
                user="root",
            )
        except Exception:  # noqa: BLE001 - observation stays honest-empty
            self.logger.warning("delivered observation failed", exc_info=True)
            return []
        output = (getattr(listing, "stdout", "") or "").splitlines()
        # The listing leads with `pwd`; every later line is
        # `<sha256>  <relative path>` from the environment working dir.
        root = (
            output[0].strip().rstrip("/")
            if output and output[0].startswith("/")
            else ""
        )
        delivered = []
        for line in output[1:]:
            digest, _, path = line.partition("  ")
            if len(digest) != 64 or not path:
                continue
            source = path.strip()
            if source.startswith("./"):
                source = source[2:]
            delivered.append({"source": f"{root}/{source}", "sha256": digest})
        return delivered

    def _write_terminal_observation(
        self,
        environment: BaseEnvironment,
        exit_code: int | None,
        delivered: list[dict[str, Any]],
    ) -> None:
        del environment  # the observation is host-side, from logs_dir
        # Schema /3 binding requires a capture summary even when the helper
        # died (e.g. the Agent timeout killed it) before writing its receipt;
        # a missing or unparseable receipt is honestly "unreadable".
        capture: dict[str, Any] = {"status": "unreadable", "policy": None, "receipt_sha256": None}
        receipt_path = self.logs_dir / "capture-receipt.json"
        if receipt_path.is_file():
            try:
                receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                capture = {
                    "status": receipt.get("status"),
                    "policy": receipt.get("policy"),
                    "receipt_sha256": self._digest_file(receipt_path),
                }
            except (OSError, ValueError):
                pass
        observation = {
            "schema": _TERMINAL_SCHEMA,
            "product": self._PRODUCT,
            "trial": self._handoff["trial"],
            "agent_executable_sha256": self._executable_sha256,
            "capture_helper_sha256": self._capture_helper["sha256"],
            "runtime_preflight": self._runtime_preflight,
            "helper_preflight": self._helper_preflight,
            "capture": capture,
            "task_dispatched": self._task_dispatched,
            "exit_code": exit_code,
            "timed_out": self._timed_out,
            "delivered": delivered,
        }
        path = self.logs_dir / _TERMINAL_FILENAME
        flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0)
        with os.fdopen(os.open(path, flags, 0o600), "w", encoding="utf-8") as output:
            output.write(json.dumps(observation, sort_keys=True) + "\n")


class OpiAgent(_OpiEvalAgent):
    """Runs the exact built Opi binary inside the task environment."""

    _PRODUCT = "opi"

    @staticmethod
    def name() -> str:
        return "opi-eval-opi"

    def version(self) -> str | None:
        return "1"

    def _agent_env(self) -> dict[str, str]:
        env = super()._agent_env()
        # An unused Linux abstract address isolates the session keyring and
        # yields connection refused: Opi's typed unavailable/env fallback.
        # This names no filesystem socket and starts no D-Bus service.
        env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:abstract={self._stage_dir()}/no-keyring"
        return env

    async def _project_configuration(
        self, environment: BaseEnvironment, stage: PurePosixPath
    ) -> None:
        # The runner's selected-connection projection owns every request
        # identity byte (P19-D02); the integration uploads it verbatim.
        config_path = self._handoff_path.parent.joinpath(
            self._handoff.get("config_path") or "config.toml"
        )
        if not config_path.is_file():
            raise _OpiEvalAgentError("handoff projected configuration is missing")
        await environment.upload_file(config_path, f"{stage}/{config_path.name}")

    def _agent_command(self, instruction: str, stage: PurePosixPath) -> str:
        agent = self._staged_executable(stage)
        quoted = shlex.quote(instruction)
        return (
            f"{agent} --config {stage}/config.toml"
            f" --model {shlex.quote(self._handoff.get('model', ''))}"
            " --json --json-compact --allow-mutating --no-trust"
            f" --trace {self._capture_root}/trace"
            f" -- {quoted}"
        )


class PiAgent(_OpiEvalAgent):
    """Runs the pinned pi runtime inside the task environment."""

    _PRODUCT = "pi"

    @staticmethod
    def name() -> str:
        return "opi-eval-pi"

    def version(self) -> str | None:
        return "1"

    async def _project_configuration(
        self, environment: BaseEnvironment, stage: PurePosixPath
    ) -> None:
        # The runner's selected-connection projection owns every request
        # identity byte (P19-D02); the integration uploads it verbatim.
        config_path = self._handoff_path.parent.joinpath(
            self._handoff.get("config_path") or "models.json"
        )
        if not config_path.is_file():
            raise _OpiEvalAgentError("handoff projected configuration is missing")
        await environment.upload_file(config_path, f"{stage}/{config_path.name}")

    def _agent_command(self, instruction: str, stage: PurePosixPath) -> str:
        agent = self._staged_executable(stage)
        provider, model = self._handoff["model"].split(":", 1)
        thinking = self._handoff.get("thinking_level", "off")
        if thinking not in ("off", "minimal", "low", "medium", "high", "xhigh", "max"):
            raise _OpiEvalAgentError("handoff thinking level is unsupported")
        quoted = shlex.quote(instruction)
        return (
            f"PI_CODING_AGENT_DIR={stage} {agent}"
            " --print --mode json --no-session --no-approve"
            " --no-extensions --no-skills --no-context-files"
            f" --thinking {thinking}"
            f" --provider {shlex.quote(provider)} --model {shlex.quote(model)}"
            f" -- {quoted}"
        )


def _regrade_manifest(root: Path) -> str:
    rows = []
    for path in sorted(root.rglob("*"), key=lambda path: path.relative_to(root).as_posix()):
        mode = path.lstat().st_mode
        if stat.S_ISREG(mode):
            rows.append(f"{path.relative_to(root).as_posix()}\n{hashlib.sha256(path.read_bytes()).hexdigest()}\n")
        elif not stat.S_ISDIR(mode):
            raise ValueError("unsafe-verifier-input")
    return hashlib.sha256("".join(rows).encode()).hexdigest()


def _regrade_artifact(path: Path) -> str:
    if path.is_symlink():
        raise ValueError("unsafe-verifier-input")
    if path.is_file():
        return hashlib.sha256(path.read_bytes()).hexdigest()
    _regrade_manifest(path)
    rows = [f"{entry.relative_to(path).as_posix()}\\n{hashlib.sha256(entry.read_bytes()).hexdigest()}"
            for entry in sorted(path.rglob("*")) if entry.is_file()]
    return hashlib.sha256("\\n".join(rows).encode()).hexdigest()


async def _regrade_docker(*args: str) -> str:
    process = await asyncio.create_subprocess_exec(
        "docker", *args, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE,
    )
    stdout, _ = await process.communicate()
    if process.returncode or len(stdout) > 65536:
        raise ValueError("cached-verifier-image-unavailable")
    return stdout.decode().strip()


class _VerifierDockerEnvironment(OwnedDockerEnvironment):
    """Use cached native images; offline regrading never builds or pulls."""

    async def _cached_image(self, name):
        identity = await _regrade_docker("image", "inspect", "--format", "{{.Id}}", name)
        if not identity.startswith("sha256:") or len(identity) != 71:
            raise ValueError("cached-verifier-image-unavailable")
        return {"name": name, "id": identity}

    async def _run_docker_compose_command(self, command, **kwargs):
        command = list(command)
        if command and command[0] == "build":
            # Both pinned starts await build without reading its return value.
            # The exact native image name must already exist from preparation.
            await self._cached_image(self._env_vars.main_image_name)
            return None
        if command and command[0] == "up":
            name = self.task_env_config.docker_image if self._use_prebuilt else self._env_vars.main_image_name
            self.regrade_image = await self._cached_image(name)
            command.extend(["--no-build", "--pull", "never"])
        if command and command[0] == "pull":
            raise ValueError("offline-verifier-pull-refused")
        return await super()._run_docker_compose_command(command, **kwargs)


async def _regrade(request_path: Path) -> None:
    import logging
    from pier.models.task.task import Task
    from pier.models.task.verifier_mode import resolve_effective_verifier_env_config
    from pier.models.trial.paths import TrialPaths, EnvironmentPaths
    from pier.trial.artifact_handler import ArtifactHandler
    from pier.verifier.verifier import Verifier

    request_bytes = request_path.read_bytes()
    request = json.loads(request_bytes)
    if request.get("schema") != "opi-eval-verifier-request/1" or request.get("runner") != 'pier':
        raise ValueError("invalid-verifier-request")
    root = Path(request["trace_root"])
    if root.resolve() != request_path.parent.resolve() / "verifier-trace":
        raise ValueError("invalid-verifier-root")
    observation = {
        "schema": "opi-eval-verifier-observation/1",
        "request_sha256": hashlib.sha256(request_bytes).hexdigest(),
        "verifier_id": request["verifier_id"], "restored": request["restored"],
        "build_context_sha256": request["build_context_sha256"],
        "image": None, "raw_result": None, "cleanup_confirmed": True, "failure": None,
    }
    environment = None
    try:
        task_path = Path(request["task_package"])
        if _regrade_manifest(task_path) != request["task_package_manifest_sha256"]:
            raise ValueError("task-package-drift")
        task = Task(task_path)
        if task.has_steps or task.name != request["task_name"]:
            raise ValueError("task-identity-drift")
        separate = request["environment_mode"] == "separate"
        env_config = resolve_effective_verifier_env_config(task.config, None)
        if separate != (env_config is not None):
            raise ValueError("lifecycle-drift")
        env_config = env_config if separate else task.config.environment
        context = task.paths.tests_dir if separate else task.paths.environment_dir
        if _regrade_manifest(context) != request["build_context_sha256"]:
            raise ValueError("verifier-build-context-drift")
        for values in (env_config.env, task.config.verifier.env):
            if any(any(part in key.upper() for part in ("KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL"))
                   for key in values):
                raise ValueError("verifier-env-credential")
        collected = Path(request["collected_root"])
        for entry in request["restored"]:
            relative = PurePosixPath(entry["destination"])
            if relative.is_absolute() or ".." in relative.parts:
                raise ValueError("unsafe-verifier-input")
            if _regrade_artifact(collected / relative) != entry["sha256"]:
                raise ValueError("restored-artifacts-tampered")
        paths = TrialPaths(root)
        paths.mkdir()
        logger = logging.getLogger("opi-eval-verifier")
        if env_config.allow_internet:
            raise ValueError("verifier-network-required")
        environment = _VerifierDockerEnvironment(
            environment_dir=context, environment_name=task.name,
            session_id=request["verifier_id"], trial_paths=paths, task_env_config=env_config,
            logger=logger, agent_install_spec=None, network_allowlist=None,
            default_user=task.config.verifier.user,
            mounts_json=[{"type": "bind", "source": str(paths.verifier_dir), "target": "/logs/verifier"}],
        )
        observation["cleanup_confirmed"] = False
        await environment.start(force_build=False)
        observation["image"] = environment.regrade_image
        container = await environment._run_docker_compose_command(["ps", "--quiet", "main"])
        actual_image = await _regrade_docker("inspect", "--type", "container", "--format", "{{.Image}}", container.stdout.strip())
        if actual_image != observation["image"]["id"]:
            raise ValueError("verifier-image-drift")
        env_paths = EnvironmentPaths.for_os(env_config.os)
        await environment.empty_dirs([env_paths.verifier_dir], chmod=True)
        handler = ArtifactHandler(artifacts=task.config.artifacts, logger=logger)
        await handler.upload_artifacts(
            environment, artifacts_dir=collected,
            source_artifacts_dir=env_paths.artifacts_dir, target_artifacts_dir=env_paths.artifacts_dir,
        )
        verifier = Verifier(task=task, trial_paths=paths, environment=environment,
                            logger=logger, skip_tests_upload=separate, verifier_env=None)
        limits = [float(request["timeout_seconds"])]
        for value in (task.config.verifier.timeout_sec, request.get("verifier_timeout_seconds")):
            if value is not None and value > 0:
                limits.append(float(value))
        result = await asyncio.wait_for(verifier.verify(), min(limits))
        raw = result.model_dump_json().encode()
        (root / "native-result.json").write_bytes(raw)
        observation["raw_result"] = {"path": "native-result.json", "sha256": hashlib.sha256(raw).hexdigest()}
    except (Exception, asyncio.CancelledError) as error:
        observation["failure"] = ("verifier-timeout" if isinstance(error, TimeoutError)
                                   else "verifier-cancelled" if isinstance(error, asyncio.CancelledError)
                                   else str(error) if isinstance(error, ValueError) else type(error).__name__)
    finally:
        if environment is not None:
            try:
                await environment.stop(delete=True)
                observation["cleanup_confirmed"] = True
            except (Exception, asyncio.CancelledError):
                observation["cleanup_confirmed"] = False
        (root / "observation.json").write_text(json.dumps(observation, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    import sys

    if len(sys.argv) != 2:
        raise SystemExit("one private verifier request is required")
    asyncio.run(_regrade(Path(sys.argv[1])))
