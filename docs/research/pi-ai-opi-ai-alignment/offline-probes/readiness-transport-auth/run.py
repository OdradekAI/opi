"""Replay the transport/auth research, not product acceptance.

Explicit network preparation is separate: prepare-npm.py. This command only
uses those verified sources, Cargo --locked --offline and loopback endpoints.
Supply an existing shared Cargo cache; it never creates a target-dir override.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--node", required=True, type=Path)
parser.add_argument("--npm", required=True, type=Path)
parser.add_argument("--pi", required=True, type=Path)
parser.add_argument("--scratch", required=True, type=Path)
parser.add_argument("--expect-node-sha256", default="d14ba95cdce1ef7dc9ad3ac74949ca5db38b27378ee30f30a23cf26f9e875a11")
args = parser.parse_args()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
if sha(args.node) != args.expect_node_sha256:
    raise SystemExit("Node executable pin mismatch; no commands executed")
preparation = json.loads((args.npm / "preparation.json").read_text())
if sha(args.pi / "package-lock.json") != preparation["lock_sha256"]:
    raise SystemExit("Pi lock pin mismatch; no commands executed")
for package in preparation["packages"]:
    for member in package["members"]:
        path = args.npm / package["package"] / Path(*Path(member["path"]).parts[1:])
        if not path.is_file() or sha(path) != member["sha256"]:
            raise SystemExit(f"Npm source pin mismatch: {path}; no commands executed")
asset = Path(__file__).resolve().parent
scratch = args.scratch.resolve()
scratch.mkdir(parents=True, exist_ok=True)
crate = scratch / "crate"
crate.mkdir(exist_ok=True)
for name in ("Cargo.toml", "Cargo.lock", "main.rs"):
    shutil.copyfile(asset / name, crate / name)
commands = []
child_env = {k: os.environ[k] for k in ("SystemRoot", "PATH") if k in os.environ}
child_env.update(TEMP=str(scratch), TMP=str(scratch))


def execute(command, timeout, output, env=None):
    result = subprocess.run([str(x) for x in command], capture_output=True,
                            timeout=timeout, env=env)
    (scratch / output).write_bytes(result.stdout)
    (scratch / (output + ".stderr")).write_bytes(result.stderr)
    commands.append({"command": [str(x) for x in command], "exit": result.returncode,
                     "stdout": output, "stderr": output + ".stderr",
                     "containment_seconds": timeout})
    (scratch / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
    if result.returncode:
        raise SystemExit(f"Command failed ({result.returncode}): {command}")
    return result.stdout


execute(["cargo", "build", "--locked", "--offline", "--manifest-path", crate / "Cargo.toml"],
        120, "cargo-build.txt")
metadata = json.loads(execute(["cargo", "metadata", "--locked", "--offline", "--format-version", "1", "--manifest-path", crate / "Cargo.toml"],
                             30, "cargo-metadata.json"))
execute(["cargo", "tree", "--locked", "--offline", "-e", "features", "--manifest-path", crate / "Cargo.toml"],
        30, "cargo-features.txt")
binary = Path(metadata["target_directory"]) / "debug" / "opi-research-transport-auth.exe"
fixtures = scratch / "fixtures"
execute([sys.executable, asset / "fixtures.py", fixtures], 90, "fixtures.txt")
data = execute([args.node, asset / "probe.mjs", args.npm, fixtures, binary], 90,
               "results.jsonl", child_env)
proxy = execute([args.node, "--experimental-strip-types", asset / "proxy-probe.mjs", args.pi],
                15, "proxy-results.jsonl", child_env)
rows = [json.loads(x) for x in data.splitlines()]
proxy_rows = [json.loads(x) for x in proxy.splitlines()]
assert len(rows) == 112 and len(proxy_rows) == 16
native_rows = [x for x in rows if x.get("outcome", {}).get("result") is not None]
assert all(x["outcome"]["exit"] == 0 for x in native_rows)
rsa = [x for x in rows if x["kind"] == "rsa"]
assert len(rsa) == 10 and all(x["reference"]["accepted"] and x["reference"]["verified"] for x in rsa)
assert sum(x["native"]["result"]["accepted"] for x in rsa) == 4
tls = {x["id"]: x for x in rows if x["kind"] == "tls"}
assert tls["h2"]["outcome"]["result"]["version"] == "HTTP/2.0"
assert tls["h1"]["outcome"]["result"]["version"] == "HTTP/1.1"
for name in ("mtls-rsa", "mtls-ec", "mtls-ed25519"):
    assert tls[name]["requests"][0]["authorized"] is True
for name in ("no-trust", "wrong-host", "expired", "mtls-missing"):
    assert tls[name]["outcome"]["result"]["ok"] is False and tls[name]["requests"] == []
assert [(x["residency"], x["value"]) for x in rows if x["kind"] == "metadata-timeout"] == [(False, 3000), (True, 0)]
for row in rows:
    if row["kind"] == "redirect" and row["backend"] == "reqwest-owner-leg":
        assert len(row["requests"]) == 1 and row["outcome"]["result"]["status"] == row["status"]
    if row["kind"] == "redirect-loop":
        assert len(row["requests"]) == (11 if row["backend"] == "reqwest-default" else min(row["n"] + 1, 21))
summary = {"status": "MECHANISM-RESULTS-RECORDED-NOT-PRODUCT-PASS", "rows": len(rows),
           "proxy_rows": len(proxy_rows), "node_sha256": sha(args.node),
           "binary_sha256": sha(binary), "lock_sha256": sha(crate / "Cargo.lock"),
           "command_count": len(commands), "commands_all_exit_zero": True,
           "registry_packages": sum((x.get("source") or "").startswith("registry+") for x in metadata["packages"]),
           "package_count": len(metadata["packages"]), "environment_keys": sorted(child_env)}
(scratch / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary))
