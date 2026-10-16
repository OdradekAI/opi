"""Research dependency preparation only; never run npm/Bun or package scripts.

Usage: python prepare-npm.py PI_PACKAGE_LOCK TEMP_DIRECTORY
The original lock's registry URL/integrity is authority. Existing archives are
checked identically; every acquisition and extracted member is logged. This
explicit preparation can use the network. Experiment processes use only these
files, synthetic inputs and loopback servers.
"""
import base64
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import sys
import tarfile
import urllib.request

lock_path, destination = Path(sys.argv[1]), Path(sys.argv[2]).resolve()
packages = json.loads(lock_path.read_text(encoding="utf-8"))["packages"]
selected = {}


def resolve(parent, name):
    current = PurePosixPath(parent)
    while True:
        candidate = str(current / "node_modules" / name)
        if candidate in packages:
            return candidate
        if str(current) == ".":
            raise RuntimeError(f"Unresolved locked dependency {parent}: {name}")
        current = current.parent


def visit(key):
    if key in selected:
        return
    value = packages[key]
    if not value.get("resolved", "").startswith("https://registry.npmjs.org/"):
        raise RuntimeError(f"Non-registry source: {key}")
    if not value.get("integrity", "").startswith("sha512-"):
        raise RuntimeError(f"Missing SHA-512 integrity: {key}")
    selected[key] = value
    for name in value.get("dependencies", {}):
        visit(resolve(key, name))


for name in ("node-fetch", "jws", "gcp-metadata", "google-auth-library"):
    visit("node_modules/" + name)
destination.mkdir(parents=True, exist_ok=True)
records = []
for key, value in sorted(selected.items()):
    archive_path = destination / "archives" / (key.replace("/", "__") + ".tgz")
    archive_path.parent.mkdir(exist_ok=True)
    fetched = not archive_path.exists()
    if fetched:
        with urllib.request.urlopen(value["resolved"], timeout=60) as response:
            body = response.read()
    else:
        body = archive_path.read_bytes()
    actual = "sha512-" + base64.b64encode(hashlib.sha512(body).digest()).decode()
    if actual != value["integrity"]:
        raise RuntimeError(f"Integrity mismatch: {key}")
    archive_path.write_bytes(body)
    members = []
    with tarfile.open(fileobj=io.BytesIO(body), mode="r:gz") as archive:
        for member in archive.getmembers():
            path = PurePosixPath(member.name)
            if not path.parts or path.parts[0] != "package" or ".." in path.parts:
                raise RuntimeError(f"Unsafe archive member: {member.name}")
            if member.isdir():
                continue
            if not member.isfile():
                raise RuntimeError(f"Non-regular archive member: {member.name}")
            data = archive.extractfile(member).read()
            target = destination / key / Path(*path.parts[1:])
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
            members.append({"path": member.name, "bytes": len(data),
                            "sha256": hashlib.sha256(data).hexdigest()})
    records.append({"package": key, "version": value["version"],
                    "url": value["resolved"], "integrity": actual,
                    "sha256": hashlib.sha256(body).hexdigest(), "bytes": len(body),
                    "network_acquired": fetched, "members": members})
manifest = {"lock_sha256": hashlib.sha256(lock_path.read_bytes()).hexdigest(),
            "destination": str(destination), "package_count": len(records),
            "network_acquisitions": sum(r["network_acquired"] for r in records),
            "packages": records}
(destination / "preparation.json").write_text(
    json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(json.dumps({k: v for k, v in manifest.items() if k != "packages"}))
