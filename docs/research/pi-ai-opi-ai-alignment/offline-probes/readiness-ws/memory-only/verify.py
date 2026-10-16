"""Prepare cached, pinned sources and run only in-memory/offline research tests."""
import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tarfile
import tomllib

HERE = pathlib.Path(__file__).resolve().parent
PARENT = HERE.parent


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", required=True, type=pathlib.Path)
    parser.add_argument("--expected-before")
    args = parser.parse_args()
    pins = json.loads((HERE / "source-files.json").read_text(encoding="utf-8"))
    frame_pin = next(row for row in pins["files"] if row["path"] == "src/protocol/frame.rs")
    if args.expected_before is not None and args.expected_before != frame_pin["before"]:
        raise SystemExit("source pin mismatch before workspace creation")
    work = args.workspace.resolve()
    if work.exists():
        raise SystemExit("workspace must not exist")

    cache = pathlib.Path.home() / ".cargo/registry/cache/index.crates.io-1949cf8c6b5b557f"
    archive = cache / "ratchet_core-1.2.1.crate"
    if sha(archive.read_bytes()) != pins["archiveSha256"]:
        raise SystemExit("cached upstream archive checksum mismatch")
    # No fetch/download path exists. Check every locked registry archive first.
    lock = tomllib.loads((HERE / "Cargo.lock").read_text(encoding="utf-8"))
    dependencies = []
    for row in lock["package"]:
        if row.get("source", "").startswith("registry+"):
            cached = cache / f"{row['name']}-{row['version']}.crate"
            digest = sha(cached.read_bytes())
            if digest != row["checksum"]:
                raise SystemExit("cached dependency checksum mismatch: " + cached.name)
            dependencies.append({"name": row["name"], "version": row["version"], "sha256": digest})
    for row in pins["files"]:
        if sha((HERE / "replacements" / row["path"]).read_bytes()) != row["after"]:
            raise SystemExit("replacement checksum mismatch: " + row["path"])

    work.mkdir(parents=True)
    with tarfile.open(archive) as tar:
        tar.extractall(work / "source", filter="data")
    source = work / "source/ratchet_core-1.2.1"
    # Verify all before states before applying any replacement.
    for row in pins["files"]:
        path = source / row["path"]
        actual = sha(path.read_bytes()) if path.exists() else None
        if actual != row["before"]:
            raise SystemExit("source checksum mismatch before patch: " + row["path"])
    for row in pins["files"]:
        path = source / row["path"]
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(HERE / "replacements" / row["path"], path)
        if sha(path.read_bytes()) != row["after"]:
            raise SystemExit("source checksum mismatch after patch: " + row["path"])
    for relative in ["Cargo.toml", "Cargo.lock", "src/lib.rs", "memory-codec-cases.json"]:
        path = work / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(HERE / relative, path)

    env = os.environ.copy()
    env["CARGO_NET_OFFLINE"] = "true"
    commands = []

    def run(command, log, expected=0):
        result = subprocess.run(command, capture_output=True, text=True, env=env)
        (work / log).write_text(result.stdout + result.stderr, encoding="utf-8")
        commands.append({"command": command, "exit": result.returncode, "expectedExit": expected})
        (work / "commands.json").write_text(json.dumps(commands, indent=2) + "\n", encoding="utf-8")
        if (expected == 0 and result.returncode != 0) or (expected != 0 and result.returncode == 0):
            raise SystemExit("unexpected subprocess result; inspect " + str(work / log))
        return result

    command = ["cargo", "test", "--locked", "--offline", "--manifest-path", str(work / "Cargo.toml"),
               "--workspace", "--lib", "readiness_memory", "--", "--nocapture"]
    result = run(command, "memory-test.txt")
    passed = [int(x) for x in re.findall(r"test result: ok\. (\d+) passed", result.stdout)]
    if sorted(passed) != [1, 13]:
        raise SystemExit("unexpected memory test inventory")

    # Directory regression: run the old 80-row summarizer only on copied local
    # files. This executes no Node/Bun client, socket harness or request code.
    fixture = work / "summary-fixture"
    (fixture / "rust").mkdir(parents=True)
    for name in ["summarize.py", "results.json", "outbound.txt"]:
        shutil.copyfile(PARENT / name, fixture / name)
    shutil.copyfile(PARENT / "rust/Cargo.lock", fixture / "rust/Cargo.lock")
    run([sys.executable, str(fixture / "summarize.py")], "summary-fixture-positive.txt")
    raw = json.loads((fixture / "results.json").read_text(encoding="utf-8"))
    raw["results"].pop()
    (fixture / "results.json").write_text(json.dumps(raw), encoding="utf-8")
    run([sys.executable, str(fixture / "summarize.py")], "summary-fixture-negative.txt", expected=1)
    summary = {
        "scope": "pure-memory only; no sockets, connections, requests or downloads",
        "memoryTestsPassed": sum(passed), "publicCodecFixtures": 12,
        "cachedRegistryPackagesVerified": len(dependencies),
        "sourceFiles": pins["files"], "commands": commands,
        "historical80Rerun": False,
    }
    (work / "verification.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"workspace": str(work), "memoryTestsPassed": sum(passed), "summaryFixturePositiveNegative": True}))


if __name__ == "__main__":
    main()
