#!/usr/bin/env python3
"""Observe exact frozen clients against one offline loopback listener.

The production Companion renders dummy-auth configs and validates pinned wire
evidence. No live endpoint, credential or ambient client profile is consulted.
Material is replaced only after every selected client passes admission.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import stat
import subprocess
import sys
import tomllib


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def pin(path: Path) -> dict:
    return {"path": str(path.resolve()), "sha256": sha(path)}


def package_digest(root: Path) -> str:
    rows = []
    for path in sorted(root.rglob("*"), key=lambda p: p.relative_to(root).as_posix()):
        mode = path.lstat().st_mode
        if stat.S_ISREG(mode):
            rows.append(f"{path.relative_to(root).as_posix()}\n{sha(path)}\n")
        elif not stat.S_ISDIR(mode):
            raise ValueError("unsafe runtime package entry")
    return hashlib.sha256("".join(rows).encode()).hexdigest()


def verify_agent(agent: dict) -> None:
    executable = Path(agent["executable"]["path"])
    if not executable.is_absolute() or executable.is_symlink() or not executable.is_file():
        raise ValueError("executable must be an absolute regular file")
    if sha(executable) != agent["executable"]["sha256"]:
        raise ValueError("executable digest differs")
    package = agent.get("runtime_package")
    if package:
        root = Path(package["path"])
        entry = Path(package["entrypoint"])
        if not root.is_absolute() or root.is_symlink() or entry.is_absolute() or ".." in entry.parts:
            raise ValueError("invalid runtime entrypoint")
        if root.joinpath(entry).resolve() != executable.resolve():
            raise ValueError("runtime entrypoint differs from executable")
        if package_digest(root) != package["manifest_sha256"]:
            raise ValueError("runtime package digest differs")


def run_client(argv: list[str], cwd: Path, env: dict, out: Path) -> int:
    # One process group bounds tools and descendants even on timeout. Output
    # is intentionally discarded: only whitelisted listener knobs are proof.
    process = subprocess.Popen(argv, cwd=cwd, env=env, start_new_session=True,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        code = process.wait(timeout=180)
    except subprocess.TimeoutExpired:
        code = -1
    finally:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        process.wait()
    (out / "command.json").write_text(json.dumps({"argv": argv, "cwd": str(cwd), "exit_code": code}, indent=2) + "\n")
    return code


def produce(args: argparse.Namespace) -> None:
    root = args.out.resolve()
    if root.exists():
        raise ValueError("proof output must be fresh")
    if os.name != "posix" or not sys.platform.startswith("linux"):
        raise ValueError("native control proof requires Linux process groups")
    config = args.config.resolve(strict=True)
    companion = args.companion.resolve(strict=True)
    material_path = args.material.resolve(strict=True)
    original = material_path.read_bytes()
    material = json.loads(original)
    document = tomllib.loads(config.read_text())
    products = sorted({subject["product"] for subject in document["subjects"]})
    if not products or any(product not in ("pi", "opi") for product in products):
        raise ValueError("unknown native product")
    for product in products:
        verify_agent(material["agents"][product])
    root.mkdir(mode=0o700)
    for product in products:
        case = root / product
        work = case / "work"
        work.mkdir(parents=True, mode=0o700)
        (work / ".env").write_text("")
        log = case / "requests.jsonl"
        env = {"PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": str(work), "PYTHONDONTWRITEBYTECODE": "1"}
        listener = subprocess.Popen([sys.executable, str(Path(__file__).with_name("scripted-provider.py")),
                                     "--listen", "127.0.0.1:0", "--request-log", str(log)],
                                    env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(listener.stdout, selectors.EVENT_READ)
                if not selector.select(timeout=15):
                    raise ValueError("loopback listener readiness timed out")
            ready = listener.stdout.readline().decode().strip()
            if not ready.startswith("listening 127.0.0.1:"):
                raise ValueError("loopback listener failed")
            endpoint = "http://" + ready.removeprefix("listening ")
            projected = subprocess.run([str(companion), "__native-control-config", "--config", str(config),
                                       "--product", product, "--base-url", endpoint],
                                      env=env, capture_output=True, text=True, check=True, timeout=30)
            projection = json.loads(projected.stdout)
            config_path = case / projection["config_filename"]
            config_path.write_text(projection["config"], encoding="utf-8")
            config_path.chmod(0o600)
            connection = projection["connection"]
            executable = material["agents"][product]["executable"]["path"]
            prompt = "Use the bash tool to run the command, then finish."
            agent_env = {**env, "no_proxy": "127.0.0.1,localhost", **projection["agent_env"]}
            if product == "pi":
                agent_env["PI_CODING_AGENT_DIR"] = str(case)
                argv = [executable, "--print", "--mode", "json", "--no-session", "--no-approve", "--no-extensions",
                        "--no-skills", "--no-context-files", "--thinking", projection["thinking_level"],
                        "--provider", connection["provider"], "--model", connection["model"], "--", prompt]
            else:
                agent_env.update({"XDG_CONFIG_HOME": str(work / ".config"), "XDG_DATA_HOME": str(work / ".local/share"),
                                  "XDG_CACHE_HOME": str(work / ".cache"), "APPDATA": str(work / ".appdata"),
                                  "DBUS_SESSION_BUS_ADDRESS": "unix:abstract=opi-eval-probe-" + hashlib.sha256(str(case).encode()).hexdigest(),
                                  "DBUS_SYSTEM_BUS_ADDRESS": "unix:abstract=opi-eval-probe-system-" + hashlib.sha256(str(case).encode()).hexdigest()})
                argv = [executable, "--config", str(config_path), "--model", connection["provider"] + ":" + connection["model"],
                        "--json", "--json-compact", "--non-interactive", "--allow-mutating", "--no-trust", "-C", str(work), "--", prompt]
            code = run_client(argv, work, agent_env, case)
            if code != 0:
                raise ValueError(f"{product} offline client failed (exit {code}); no proof published")
        finally:
            try:
                os.killpg(listener.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            listener.wait()
            listener.stdout.close()
        verify_agent(material["agents"][product])
        package = material["agents"][product].get("runtime_package")
        evidence = {"schema": "opi-eval-output-control/1", "product": product,
                    "executable_sha256": material["agents"][product]["executable"]["sha256"],
                    "runtime_package_manifest_sha256": package["manifest_sha256"] if package else None,
                    "runtime_package_entrypoint": package["entrypoint"] if package else None,
                    "connection": connection, "model_controls": projection["model_controls"],
                    "probe_base_url": endpoint, "exit_code": code, "config": pin(config_path), "requests": pin(log)}
        evidence_path = case / "output-control.json"
        evidence_path.write_text(json.dumps(evidence, indent=2, sort_keys=True) + "\n")
        material["agents"][product]["output_control_evidence"] = pin(evidence_path)
    projected_material = root / "material.json"
    projected_material.write_text(json.dumps(material, indent=2, sort_keys=True) + "\n")
    admission = subprocess.run([str(companion), "__native-control-admit", "--config", str(config),
                               "--material", str(projected_material)], capture_output=True, text=True, timeout=60)
    (root / "admission.json").write_text(json.dumps({"exit_code": admission.returncode, "stderr": admission.stderr}) + "\n")
    if admission.returncode:
        raise ValueError("exact-client wire admission rejected: " + admission.stderr.strip())
    if material_path.read_bytes() != original:
        raise ValueError("material changed while probing")
    temporary = material_path.with_name(material_path.name + ".control-proof.tmp")
    with temporary.open("xb") as output:
        output.write(projected_material.read_bytes())
    os.replace(temporary, material_path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("companion", "config", "material", "out"):
        parser.add_argument("--" + name, type=Path, required=True)
    try:
        produce(parser.parse_args())
    except (ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"native-control-proof: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
