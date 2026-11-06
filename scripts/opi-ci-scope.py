#!/usr/bin/env python3
"""Select product CI and optional native-platform smoke from an exact Git delta."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path


RUNNERS = {"linux": "ubuntu-latest", "windows": "windows-latest", "macos": "macos-latest"}


def classify(paths: list[str]) -> dict[str, bool]:
    """Documentation and development-only eval changes do not build the product.

    Known platform-sensitive surfaces add native smoke, not full regression.
    This is a conservative routing aid; owners still select extra platforms for
    behavioral changes whose impact is not visible in a pathname.
    """
    product = []
    for path in paths:
        if path.startswith(("crates/opi-eval/", ".agents/skills/opi-eval/")):
            continue
        if path.endswith(".md") or path in {"LICENSE", ".gitignore"}:
            continue
        product.append(path.lower())
    windows = macos = False
    for path in product:
        shared = (
            path in {"cargo.toml", "cargo.lock"}
            or path.endswith(("/cargo.toml", "/build.rs"))
            or path.startswith((".github/", ".cargo/", "crates/opi-sandbox/", "crates/opi-tui/"))
            or any(word in path for word in ("execution", "native_keyring", "auth_process", "process_supervisor"))
        )
        windows |= shared or path.endswith(".ps1") or "windows" in path or "win32" in path
        macos |= shared or any(word in path for word in ("macos", "darwin", "seatbelt"))
    return {"rust": bool(product), "windows": windows, "macos": macos}


def selected_matrix(linux: bool, windows: bool, macos: bool) -> dict[str, list[str]]:
    selected = [RUNNERS[name] for name, enabled in (("linux", linux), ("windows", windows), ("macos", macos)) if enabled]
    if not selected:
        raise ValueError("select at least one platform")
    return {"os": selected}


def git_paths(base: str, head: str) -> list[str]:
    for value in (base, head):
        if not re.fullmatch(r"[0-9a-f]{40}", value):
            raise ValueError("base and head must be full lowercase commit SHAs")
    if base == "0" * 40:
        command = ["git", "ls-tree", "-r", "--name-only", "-z", head]
    else:
        command = ["git", "diff", "--name-only", "--no-renames", "-z", base, head]
    result = subprocess.run(command, check=True, capture_output=True)
    check = (["git", "show", "--format=", "--check", head] if base == "0" * 40
             else ["git", "diff", "--check", base, head])
    subprocess.run(check, check=True)
    return [path for path in result.stdout.decode("utf-8").split("\0") if path]


def emit(values: dict[str, object], output: str | None) -> None:
    print(json.dumps(values, sort_keys=True))
    if output:
        with Path(output).open("a", encoding="utf-8") as stream:
            for key, value in values.items():
                stream.write(f"{key}={json.dumps(value, separators=(',', ':'))}\n")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    scope = commands.add_parser("scope")
    scope.add_argument("--base", required=True)
    scope.add_argument("--head", required=True)
    scope.add_argument("--output")
    matrix = commands.add_parser("matrix")
    matrix.add_argument("--ref", required=True)
    for platform in RUNNERS:
        matrix.add_argument(f"--{platform}", choices=("true", "false"), default="false")
    matrix.add_argument("--output")
    args = parser.parse_args()
    try:
        if args.command == "scope":
            emit(classify(git_paths(args.base, args.head)), args.output)
        else:
            if not re.fullmatch(r"[0-9a-f]{40}", args.ref):
                raise ValueError("ref must be a full lowercase commit SHA")
            emit({"matrix": selected_matrix(args.linux == "true", args.windows == "true", args.macos == "true")}, args.output)
    except (ValueError, subprocess.CalledProcessError, UnicodeError) as error:
        parser.exit(2, f"CI selection failed: {error}\n")


if __name__ == "__main__":
    main()
