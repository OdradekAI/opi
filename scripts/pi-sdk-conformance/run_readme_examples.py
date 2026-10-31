#!/usr/bin/env python3
"""ONB-02 (task 20.57): package-shipped README example proof.

Packages ``opi-ai`` with Cargo, extracts the artifact into a new
workspace-external scratch, proves the three onboarding examples ship inside
the ``.crate`` byte-exactly as the bilingual READMEs embed them, then builds
a fresh ordinary consumer whose binaries are those packaged example sources
against only ``opi-ai`` plus documented ordinary runtime/Serde/stream
dependencies. The consumer is compiled offline under an isolated controlled
environment; no credentials exist, so no live provider call can start. The
controlled-peer execution of the same request/final-result/continuation
flows is owned by ``crates/opi-ai/tests/external_sdk.rs``. This runner never
publishes, patches the root manifest, resolves from the network, or builds
cross-platform binaries.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Any

import run_early_package_proof as shared
from package_slice import check_cargo_configs, controlled_environment

HARNESS = shared.HARNESS
REPO = shared.REPO

EXAMPLES = ("quick_start.rs", "quick_start_stream.rs", "tool_loop.rs")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--offline", action="store_true",
                        help="required offline mode marker")
    parser.add_argument("--scratch", required=True,
                        help="new workspace-external scratch directory")
    parser.add_argument("--target-dir", default=os.environ.get("CARGO_TARGET_DIR"),
                        help="existing marked workspace cache (or CARGO_TARGET_DIR)")
    args = parser.parse_args()
    if not args.offline:
        parser.error("--offline is required; this proof never resolves from the network")
    if not args.target_dir:
        parser.error("--target-dir or CARGO_TARGET_DIR must identify the existing workspace cache")

    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    try:
        env = controlled_environment(dict(os.environ), target)
        configs = check_cargo_configs(env, [REPO, scratch / "consumer", scratch / "package"])
        shared.validate_paths(REPO, scratch, target)
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env.setdefault("CARGO_BUILD_JOBS", "4")

    evidence: dict[str, Any] = {
        "family": "readme-examples",
        "scope": ("packaged onboarding examples ship byte-exactly and compile "
                  "from a fresh workspace-external consumer (task 20.57, ONB-02); "
                  "the controlled-peer execution lives in external_sdk.rs"),
        "target": str(target),
        "cargo_configs": configs,
        "steps": [],
        "verdict": "FAIL",
    }

    def run(name: str, command: list[str], cwd: Path) -> str:
        started = time.monotonic()
        result = subprocess.run(
            command, cwd=cwd, env=env, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=3600,
        )
        (scratch / f"{name}.stdout").write_text(result.stdout, encoding="utf-8")
        (scratch / f"{name}.stderr").write_text(result.stderr, encoding="utf-8")
        evidence["steps"].append({
            "name": name, "command": command, "exit": result.returncode,
            "elapsed_seconds": round(time.monotonic() - started, 2),
        })
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}; see scratch logs")
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        evidence["rustc"] = run("rustc-version", ["rustc", "-vV"], REPO)
        evidence["cargo"] = run("cargo-version", ["cargo", "-vV"], REPO)
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        version = tomllib.loads(
            (REPO / "Cargo.toml").read_text(encoding="utf-8")
        )["workspace"]["package"]["version"]
        prefix = f"opi-ai-{version}"
        run("package", ["cargo", "package", "-p", "opi-ai", "--allow-dirty", "--no-verify",
                        "--offline", "--locked"], REPO)
        archive = scratch / f"{prefix}.crate"
        archive.write_bytes((target / "package" / archive.name).read_bytes())
        evidence["package"] = {
            "file": archive.name,
            "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
        }
        package = shared.unpack_package(archive, scratch / "package", prefix)

        # The artifact ships the three onboarding examples byte-exactly as
        # the workspace sources, and both manuals embed those exact bytes.
        examples: dict[str, str] = {}
        for name in EXAMPLES:
            packaged = package / "examples" / name
            source = packaged.read_bytes()
            examples[name] = hashlib.sha256(source).hexdigest()
            expected = (REPO / "crates/opi-ai/examples" / name).read_bytes()
            if source != expected:
                raise ValueError(f"packaged example {name} differs from the workspace source")
        evidence["packaged_examples"] = examples
        evidence["manuals_embed_exact_sources"] = {}
        for manual in ("README.md", "README.zh.md"):
            text = (package / manual).read_text(encoding="utf-8")
            embedded = all(
                source.decode("utf-8").rstrip("\n") in text
                for source in (
                    (package / "examples" / name).read_bytes() for name in EXAMPLES
                )
            )
            evidence["manuals_embed_exact_sources"][manual] = embedded
            if not embedded:
                raise ValueError(f"{manual} does not embed the exact packaged example sources")

        consumer = scratch / "consumer"
        shutil.copytree(
            HARNESS / "consumer-readme",
            consumer,
            ignore=shutil.ignore_patterns("target", "Cargo.lock"),
        )
        manifest = consumer / "Cargo.toml"
        text = manifest.read_text(encoding="utf-8")
        manifest.write_text(
            text.replace('"@PACKAGE@"', json.dumps(package.as_posix())),
            encoding="utf-8", newline="\n",
        )
        # The consumer's binaries are the packaged example sources themselves.
        (consumer / "src" / "bin").mkdir(parents=True)
        for name in EXAMPLES:
            shutil.copyfile(package / "examples" / name, consumer / "src" / "bin" / name)
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        lock = tomllib.loads((consumer / "Cargo.lock").read_text(encoding="utf-8"))
        root_names = {
            entry.get("name") for entry in lock.get("package", [])
        }
        for forbidden in ("opi-agent", "opi-coding-agent", "opi-protocol", "opi-tui"):
            if forbidden in root_names:
                raise ValueError(f"consumer graph must not contain {forbidden}")
        run("consumer-check", ["cargo", "check", "--offline", "--locked", "--bins"], consumer)
        evidence["consumer_dependencies"] = sorted(root_names)
        evidence["verdict"] = "PASS"
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(f"readme example proof failed: {exc}", file=sys.stderr)
        return 1
    finally:
        if leased:
            subprocess.run(
                lease + ["stop", "--target", str(target), "--pid", str(os.getpid())],
                cwd=REPO, capture_output=True, text=True,
            )
        out = HARNESS / "evidence" / "readme-examples-proof.json"
        out.write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"evidence: {out}")


if __name__ == "__main__":
    raise SystemExit(main())
