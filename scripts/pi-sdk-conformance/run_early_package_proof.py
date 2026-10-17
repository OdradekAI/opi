"""Early ordinary external-consumer package proof (task 20.8, DEL-04).

End-to-end orchestration:

1. verify + materialize the closure (``prepare_closure``) — namespace
   family, Unicode-16 generation (twice, determinism-checked), packaging
   into a local sparse registry in dependency order;
2. copy the ordinary consumer crate to a fresh workspace-external scratch
   directory with a fresh ``CARGO_HOME`` and its own target directory;
3. build and run the consumer against the running local registry — the
   consumer resolves the family by registry identity, uses ordinary
   upstream dependencies from crates.io alongside it, and must report
   PASS on the frozen leaf/Unicode/coexistence battery;
4. write the full evidence bundle.

Reused at 20.57 to prove the final assembled package behavior. Registry
publication remains a separately authorized step; this proof uses the local
sparse registry only.
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parents[1]
sys.path.insert(0, str(HARNESS))

from sparse_registry import Registry  # noqa: E402

DEFAULT_PORT = 8173


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True, help="fresh scratch root (created)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    parser.add_argument("--skip-prepare", action="store_true",
                        help="red-run: prove the consumer fails against an empty registry")
    args = parser.parse_args()

    scratch = Path(args.scratch).resolve()
    if scratch.exists():
        shutil.rmtree(scratch)
    scratch.mkdir(parents=True)
    registry = scratch / "registry"
    registry.mkdir()
    (registry / "manifest.json").write_text("[]\n")

    index_url = f"sparse+http://127.0.0.1:{args.port}/index/"
    server = Registry(registry, args.port)
    threading.Thread(target=server.serve, daemon=True).start()
    time.sleep(0.5)

    env = dict(os.environ)
    env.update({"no_proxy": "127.0.0.1,localhost", "NO_PROXY": "127.0.0.1,localhost"})

    evidence = {"task": "20.8 early ordinary external-consumer package proof",
                "port": args.port, "skip_prepare": args.skip_prepare, "steps": []}

    if not args.skip_prepare:
        started = time.time()
        result = subprocess.run(
            [sys.executable, str(HARNESS / "prepare_closure.py"),
             "--scratch", str(scratch / "prepare"),
             "--registry", str(registry),
             "--index-url", index_url, "--port", str(args.port)],
            env=env, capture_output=True, text=True, timeout=3600)
        step = {"name": "prepare_closure", "exit": result.returncode,
                "elapsed_seconds": round(time.time() - started, 1)}
        if result.returncode != 0:
            step["stderr_tail"] = result.stderr[-4000:]
            evidence["steps"].append(step)
            (scratch / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
            raise SystemExit(f"prepare failed: {result.stderr[-2000:]}")
        evidence["steps"].append(step)
        prepare_evidence = scratch / "prepare/prepare-evidence.json"
        evidence["prepare"] = json.loads(prepare_evidence.read_text())

        # Freeze/verify the derived Unicode probes (committed copy must match).
        derived = evidence["prepare"]["unicode_probes"]
        frozen_probes = HARNESS / "frozen/unicode16-probes.json"
        if frozen_probes.is_file():
            committed = json.loads(frozen_probes.read_text())
            if committed != derived:
                raise SystemExit("committed unicode16 probes drifted from regeneration")
        else:
            frozen_probes.parent.mkdir(exist_ok=True)
            frozen_probes.write_text(json.dumps(derived, indent=2) + "\n")
            raise SystemExit("unicode16 probes were absent; committed the regenerated copy — "
                             "rerun to verify stability")

    # Fresh workspace-external ordinary consumer -------------------------------
    consumer = scratch / "consumer"
    shutil.copytree(HARNESS / "consumer", consumer)
    shutil.copytree(HARNESS / "frozen", consumer / "frozen")
    (consumer / "Cargo.lock").unlink(missing_ok=True)
    cargo_config = consumer / ".cargo"
    cargo_config.mkdir(exist_ok=True)
    (cargo_config / "config.toml").write_text(
        f"[registries.opi-vendor]\nindex = \"{index_url}\"\n")

    cargo_home = scratch / "consumer-cargo-home"
    cargo_home.mkdir()
    target_dir = scratch / "consumer-target"
    run_env = dict(env)
    run_env.update({
        "CARGO_HOME": str(cargo_home),
        "CARGO_TARGET_DIR": str(target_dir),
        "CARGO_INCREMENTAL": "0",
    })

    started = time.time()
    build = subprocess.run(["cargo", "build", "--release"], cwd=consumer,
                           env=run_env, capture_output=True, text=True, timeout=3600)
    evidence["steps"].append({"name": "consumer cargo build --release",
                              "exit": build.returncode,
                              "elapsed_seconds": round(time.time() - started, 1)})
    if build.returncode != 0:
        evidence["consumer_build_stderr_tail"] = build.stderr[-6000:]
        (scratch / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
        raise SystemExit("consumer build failed (see evidence)")

    run = subprocess.run([str(target_dir / "release/opi-sdk-conformance-consumer")],
                         cwd=consumer, env=run_env, capture_output=True, text=True, timeout=300)
    evidence["steps"].append({"name": "consumer battery run", "exit": run.returncode})
    verdict = None
    try:
        report = json.loads(run.stdout)
        evidence["consumer_report"] = report
        verdict = report.get("verdict")
    except json.JSONDecodeError:
        evidence["consumer_stdout"] = run.stdout[-4000:]
        evidence["consumer_stderr"] = run.stderr[-4000:]

    evidence["verdict"] = verdict if verdict else "FAIL"
    (scratch / "evidence.json").write_text(json.dumps(evidence, indent=2) + "\n")
    print(json.dumps({"verdict": evidence["verdict"], "port": args.port,
                      "scratch": str(scratch)}, indent=1))
    if args.skip_prepare:
        # Red-run: resolution failure against the empty registry is the
        # expected outcome; anything else is a harness defect.
        expected_red = build.returncode != 0
        print(json.dumps({"red_run_expected_failure": expected_red}))
        raise SystemExit(0 if expected_red else 1)
    raise SystemExit(0 if evidence["verdict"] == "PASS" else 1)


if __name__ == "__main__":
    main()
