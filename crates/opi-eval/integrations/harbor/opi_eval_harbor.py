"""opi-eval pinned Harbor integration (P19-D01).

This module is the upstream integration opi-eval selects on the runner
argv (``--agent opi_eval_harbor:OpiAgent`` / ``PiAgent``). Harbor imports
it in its own process and invokes one Agent *inside the task
environment*: the selected Agent's file and shell tools operate there,
and the same upstream trial's verifier grades that environment after the
Agent phase. An independent host-side Agent run followed by an unrelated
job is not this contract.

The integration is driven exclusively by the ``OPI_EVAL_NATIVE_HANDOFF``
environment variable the opi-eval runner exports for the dispatched job.
The handoff is a typed JSON document (schema ``opi-eval-native-handoff/3``)
carrying the exact built Agent executable, the projected selected
connection (the exact configuration bytes and agent environment rendered
by the runner from the frozen experiment contract), and the credential
material; none of it is read from ambient user configuration, and the
integration never improvises or falls back to another provider, protocol,
or model. Credentials are applied only to the Agent process inside the
task environment and removed before the verifier phase runs.

After the Agent phase the integration writes a typed terminal observation
(``opi-eval-terminal.json`` next to its Agent logs): the observed exit
code, whether the Agent hit its timeout, the sha256 of the exact Agent
executable it executed, and the bounded manifest of files the Agent
delivered in the environment working directory. The opi-eval result
reader binds the imported job against this observation: a substituted
artifact or a mismatched Agent identity rejects the score even when the
job exits zero.

The module uses only the pinned Harbor v0.22.0 ``BaseAgent`` surface
(setup/run, environment exec/upload) and the Python standard library.
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import shlex
import stat
from pathlib import Path, PurePosixPath
from typing import Any, override

from harbor.agents.base import BaseAgent
from harbor.environments.base import BaseEnvironment
from harbor.models.agent.context import AgentContext

_HANDOFF_ENV = "OPI_EVAL_NATIVE_HANDOFF"
_HANDOFF_SCHEMA = "opi-eval-native-handoff/3"
_TERMINAL_SCHEMA = "opi-eval-native-agent-terminal/1"
_TERMINAL_FILENAME = "opi-eval-terminal.json"

# Where the exact built Agent executable and its projected configuration
# are staged inside the task environment. The directory is removed again
# before the verifier phase (credential cleanup): the verifier receives
# neither provider credentials nor unrelated host files.
_ENV_STAGE_DIR = PurePosixPath("/opi-eval-agent")

# Bounded delivered-artifact observation: at most this many regular files
# under the environment working directory, each at most this many bytes.
_MAX_DELIVERED_FILES = 256
_MAX_DELIVERED_BYTES = 64 * 1024 * 1024


class _OpiEvalAgentError(RuntimeError):
    """Raised for integration contract violations (fail-closed)."""


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

    async def setup(self, environment: BaseEnvironment) -> None:
        # Verify the entire runtime before any task-environment upload.
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
            await self._project_configuration(environment, stage)
        except BaseException:  # Cleanup also applies when the runner cancels setup.
            await self._cleanup(environment, stage)
            raise

    async def _project_configuration(
        self, environment: BaseEnvironment, stage: PurePosixPath
    ) -> None:
        raise NotImplementedError

    def _agent_command(self, instruction: str, stage: PurePosixPath) -> str:
        raise NotImplementedError

    def _agent_env(self) -> dict[str, str]:
        # The projected agent environment from the handoff only: never
        # ambient user values.
        return dict(self._handoff.get("agent_env") or {})

    @override
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
            if self._capture_mounted:
                if any(path.is_symlink() for path in (self.logs_dir, *self.logs_dir.parents)):
                    raise _OpiEvalAgentError("unsafe mounted capture directory")
                self.logs_dir.mkdir(parents=True, exist_ok=True)
                if any((self.logs_dir / name).exists() or (self.logs_dir / name).is_symlink()
                       for name in ("stdout.log", "stderr.log", "trace")):
                    raise _OpiEvalAgentError("mounted capture destination already exists")
                self._capture_root = PurePosixPath("/logs/agent")
        except BaseException:
            await self._cleanup(environment, stage)
            raise
        exit_code = 0
        cancelled = False
        # The pinned Docker exec uses Bash. Job control gives this launch its
        # own process group; the private pid file lives only until cleanup.
        command = (
            "set -m\n"
            f"{self._agent_command(instruction, stage)}"
            f" > {shlex.quote(f'{self._capture_root}/stdout.log')}"
            f" 2> {shlex.quote(f'{self._capture_root}/stderr.log')} &\n"
            "opi_eval_pid=$!\n"
            f"printf '%s\\n' \"$opi_eval_pid\" > {shlex.quote(f'{stage}/agent.pid')}\n"
            "wait \"$opi_eval_pid\""
        )
        launch = asyncio.create_task(environment.exec(
            command=f"bash -c {shlex.quote(command)}",
            env=self._agent_env(),
        ))
        try:
            result = await asyncio.shield(launch)
            exit_code = int(result.return_code)
        except asyncio.CancelledError:
            cancelled = True
            if not launch.cancelled():
                # Keep the transport alive to observe the actual exit. Killing
                # its client alone can leave the Agent running in the container.
                async with asyncio.timeout(5):
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
            if "Timeout" in name:
                self._timed_out = True
                exit_code = 124
            else:
                exit_code = 125
            self.logger.warning("agent phase ended with %s", name)
        finally:
            try:
                await self._capture_logs(environment, stage)
                delivered = await self._observe_delivered(environment)
            finally:
                await self._cleanup(environment, stage)
            if not cancelled:
                self._write_terminal_observation(environment, exit_code, delivered)
        if exit_code not in (0,):
            # The Agent's own non-zero exit or timeout is a graded Agent
            # outcome: the verifier still grades the environment. Raise
            # only the pinned runner's classified error so the trial
            # records it (NonZeroAgentExitCodeError family).
            from harbor.agents.installed.base import NonZeroAgentExitCodeError

            raise NonZeroAgentExitCodeError(
                f"opi-eval agent exited with {exit_code}"
                + (" (timeout)" if self._timed_out else "")
            )

    async def _capture_logs(self, environment: BaseEnvironment, stage: PurePosixPath) -> None:
        if self._capture_mounted:
            return
        self.logs_dir.mkdir(parents=True, exist_ok=True)
        for name in ("stdout.log", "stderr.log"):
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

    def _write_terminal_observation(
        self, environment: BaseEnvironment, exit_code: int, delivered: list[dict[str, Any]]
    ) -> None:
        del environment
        observation = {
            "schema": _TERMINAL_SCHEMA,
            "product": self._PRODUCT,
            "agent_executable_sha256": self._executable_sha256,
            "exit_code": exit_code,
            "timed_out": self._timed_out,
            "delivered": delivered,
        }
        (self.logs_dir / _TERMINAL_FILENAME).write_text(
            json.dumps(observation, sort_keys=True) + "\n", encoding="utf-8"
        )

    async def _observe_delivered(self, environment: BaseEnvironment) -> list[dict[str, Any]]:
        """Bounded manifest of files in the environment working directory."""
        try:
            listing = await environment.exec(
                command=(
                    "pwd && find . -maxdepth 3 -type f "
                    f"-size -{_MAX_DELIVERED_BYTES}c -print0 "
                    "| xargs -0 sha256sum | head -n "
                    f"{_MAX_DELIVERED_FILES}"
                ),
            )
        except Exception:  # noqa: BLE001 - observation stays honest-empty
            self.logger.warning("delivered observation failed", exc_info=True)
            return []
        output = (getattr(listing, "stdout", "") or "").splitlines()
        # The listing leads with `pwd`; every later line is
        # `<sha256>  <relative path>` from the environment working dir.
        root = output[0].strip().rstrip("/") if output and output[0].startswith("/") else ""
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


class OpiAgent(_OpiEvalAgent):
    """Runs the exact built Opi binary inside the task environment."""

    _PRODUCT = "opi"

    @staticmethod
    @override
    def name() -> str:
        return "opi-eval-opi"

    @override
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
    @override
    def name() -> str:
        return "opi-eval-pi"

    @override
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
        quoted = shlex.quote(instruction)
        return (
            f"PI_CODING_AGENT_DIR={stage} {agent}"
            " --print --mode json --no-session --no-approve"
            " --no-extensions --no-skills --no-context-files --thinking off"
            f" --provider {shlex.quote(provider)} --model {shlex.quote(model)}"
            f" -- {quoted}"
        )
