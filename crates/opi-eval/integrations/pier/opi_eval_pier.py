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
(schema ``opi-eval-native-handoff/2``): the exact built Agent executable,
the projected selected connection, and the credential material. None of
it is read from ambient user configuration, and the integration never
improvises or falls back to another provider, protocol, or model.
Credentials are applied only to the Agent process inside the task
environment and removed before the verifier phase runs.

After the Agent phase the integration writes a typed terminal observation
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

import hashlib
import json
import os
import shlex
from pathlib import Path, PurePosixPath
from typing import Any

from pier.agents.base import BaseAgent
from pier.environments.base import BaseEnvironment
from pier.models.agent.context import AgentContext

_HANDOFF_ENV = "OPI_EVAL_NATIVE_HANDOFF"
_HANDOFF_SCHEMA = "opi-eval-native-handoff/2"
_TERMINAL_SCHEMA = "opi-eval-native-agent-terminal/1"
_TERMINAL_FILENAME = "opi-eval-terminal.json"

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

    async def setup(self, environment: BaseEnvironment) -> None:
        # Upload the exact built Agent executable and its projected
        # configuration into the task environment. The staging directory
        # is private to the Agent phase and removed again in run().
        stage = self._stage_dir()
        executable = Path(self._handoff["agent_executable"])
        if not executable.is_file():
            raise _OpiEvalAgentError("handoff agent executable is missing")
        await environment.exec(
            command=f"mkdir -p {shlex.quote(str(stage))} && chmod 700 {shlex.quote(str(stage))}",
            user="root",
        )
        await environment.upload_file(executable, f"{stage}/agent")
        await environment.exec(
            command=f"chmod 700 {shlex.quote(f'{stage}/agent')}",
            user="root",
        )
        await self._project_configuration(environment, stage)

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

    async def run(
        self,
        instruction: str,
        environment: BaseEnvironment,
        context: AgentContext,
    ) -> None:
        stage = self._stage_dir()
        exit_code = 0
        try:
            result = await environment.exec(
                command=self._agent_command(instruction, stage),
                env=self._agent_env(),
            )
            exit_code = int(getattr(result, "return_code", 0) or 0)
        except Exception as exc:  # noqa: BLE001 - settle every terminal shape
            name = type(exc).__name__
            if "Timeout" in name:
                self._timed_out = True
                exit_code = 124
            else:
                exit_code = 125
            self.logger.warning("agent phase ended with %s", name)
        finally:
            delivered = await self._observe_delivered(environment)
            await self._cleanup(environment, stage)
            self._write_terminal_observation(environment, exit_code, delivered)
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

    async def _cleanup(self, environment: BaseEnvironment, stage: PurePosixPath) -> None:
        # Credential cleanup before the verifier phase: the staging
        # directory carried the projected configuration and credential
        # material; the pristine verifier must never see them.
        try:
            await environment.exec(
                command=f"rm -rf {shlex.quote(str(stage))}",
                user="root",
            )
        except Exception:  # noqa: BLE001 - cleanup is best effort, logged
            self.logger.warning("opi-eval stage cleanup failed", exc_info=True)

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
        exit_code: int,
        delivered: list[dict[str, Any]],
    ) -> None:
        del environment  # the observation is host-side, from logs_dir
        observation = {
            "schema": _TERMINAL_SCHEMA,
            "product": self._PRODUCT,
            "agent_executable_sha256": self._handoff.get("agent_executable_sha256"),
            "exit_code": exit_code,
            "timed_out": self._timed_out,
            "delivered": delivered,
        }
        (self.logs_dir / _TERMINAL_FILENAME).write_text(
            json.dumps(observation, sort_keys=True) + "\n", encoding="utf-8"
        )


class OpiAgent(_OpiEvalAgent):
    """Runs the exact built Opi binary inside the task environment."""

    _PRODUCT = "opi"

    @staticmethod
    def name() -> str:
        return "opi-eval-opi"

    def version(self) -> str | None:
        return "1"

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
        agent = f"{stage}/agent"
        quoted = shlex.quote(instruction)
        return (
            f"{agent} --config {stage}/config.toml"
            f" --model {shlex.quote(self._handoff.get('model', ''))}"
            " --json --json-compact --allow-mutating --no-trust"
            f" --trace {stage}/trace"
            f" {quoted}"
        )


class PiAgent(_OpiEvalAgent):
    """Runs the exact built pi bundle inside the task environment."""

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
        agent = f"{stage}/agent"
        quoted = shlex.quote(instruction)
        return (
            f"PI_CODING_AGENT_DIR={stage} {agent}"
            " --print --mode json --no-session --no-approve"
            f" --provider scripted --model {shlex.quote(self._handoff.get('model', '').split(':', 1)[-1])}"
            f" {quoted}"
        )
