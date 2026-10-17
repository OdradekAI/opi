"""Minimal sparse-registry server backing the opi-vendor local registry.

Serves the cargo sparse-index protocol over localhost HTTP from a plain
directory layout described by ``manifest.json``:

    <root>/manifest.json   [{"name", "vers", "deps": [...], "features": {...}}]
    <root>/crates/*.crate  packaged archives (sha256 recorded in the index)

Dependency rows in ``manifest.json``:

    {"name": <package name>, "req": <version req>, "ours": bool,
     "key": <toml dep key when renamed>, "optional": bool,
     "default_features": bool, "kind": "normal"|"dev"|"build",
     "features": [...]}

``ours=true`` binds the dependency to this registry (the index entry carries
this registry's URL); otherwise the dependency resolves from crates.io.
Renamed keys emit ``explicit_name_in_toml`` so fetched packages expose the
same extern crate names as their manifests.

The index is rebuilt from the manifest on every request, so the orchestrator
can grow the registry incrementally by rewriting ``manifest.json``.
"""

import hashlib
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


def sparse_paths(name: str):
    """Cargo sparse-index path variants (dash and underscore normalization)."""
    variants = set()
    for base in (name, name.replace("-", "_")):
        if len(base) <= 3:
            variants.add(Path(base))
        elif len(base) == 4:
            variants.add(Path(base[:1], base))
        else:
            variants.add(Path(base[:2], base[2:4], base))
    return variants


class Registry:
    def __init__(self, root: Path, port: int):
        self.root = root
        self.port = port
        self.dl = f"http://127.0.0.1:{port}/crates/{{crate}}-{{version}}.crate"
        self.index_url = f"sparse+http://127.0.0.1:{port}/index/"

    def build_index(self) -> None:
        index = self.root / "index"
        index.mkdir(parents=True, exist_ok=True)
        (index / "config.json").write_text(json.dumps({"dl": self.dl}))
        for stale in index.rglob("*"):
            if stale.is_file() and stale.name != "config.json":
                stale.unlink()
        for spec in json.loads((self.root / "manifest.json").read_text(encoding="utf-8")):
            name, version = spec["name"], spec["vers"]
            crate = self.root / "crates" / f"{name}-{version}.crate"
            cksum = hashlib.sha256(crate.read_bytes()).hexdigest()
            deps = []
            for d in spec.get("deps", []):
                dep = {
                    "name": d["name"],
                    "req": d.get("req", "*"),
                    "features": d.get("features", []),
                    "optional": d.get("optional", False),
                    "default_features": d.get("default_features", True),
                    "target": d.get("target"),
                    "kind": d.get("kind", "normal"),
                    # Family deps bind to this registry; ordinary deps must
                    # explicitly point at crates.io or cargo resolves them
                    # against this registry and fails.
                    # Ordinary deps must resolve from the DEFAULT crates.io
                    # source; the canonical SourceId spelling is the git
                    # index URL, which cargo matches to its default source
                    # (a sparse spelling would create a duplicate source).
                    # Ordinary deps carry the sparse crates.io URL. Note:
                    # cargo 1.97 canonicalizes its default crates.io source
                    # as the git-index SourceId, so these deps form a second
                    # (identical-content) source in the proof graph. On a
                    # real single registry every dependency resolves from
                    # one source; this duplication is an artifact of the
                    # localhost proof registry only.
                    "registry": self.index_url if d.get("ours")
                    else "sparse+https://index.crates.io/",
                }
                if d.get("key") and d["key"] != d["name"]:
                    dep["explicit_name_in_toml"] = d["key"]
                deps.append(dep)
            # v1 "features" cannot express "dep:" or weak "?" syntax;
            # those features (and features referencing them) move to
            # "features2", matching crates.io-generated index placement.
            plain, advanced = {}, {}
            for fname, refs in spec.get("features", {}).items():
                if any(str(ref).startswith("dep:") or "?" in str(ref).partition("/")[0]
                       for ref in refs):
                    advanced[fname] = refs
                else:
                    plain[fname] = refs
            changed = True
            while changed:
                changed = False
                for fname in list(plain):
                    refs = plain[fname]
                    if any(str(ref).partition("/")[0] in advanced for ref in refs):
                        advanced[fname] = refs
                        del plain[fname]
                        changed = True
            entry = {
                "name": name,
                "vers": version,
                "deps": deps,
                "cksum": cksum,
                "features": plain,
                "yanked": False,
                "v": 2,
            }
            if advanced:
                entry["features2"] = advanced
            line = json.dumps(entry, separators=(",", ":")) + "\n"
            for rel in sparse_paths(name):
                target = index / rel
                target.parent.mkdir(parents=True, exist_ok=True)
                with open(target, "a", encoding="utf-8") as out:
                    out.write(line)

    def serve(self) -> None:
        self.build_index()
        reg = self

        class Handler(BaseHTTPRequestHandler):
            protocol_version = "HTTP/1.1"

            def log_message(self, *args):
                pass

            def do_GET(self):
                try:
                    self._get()
                except Exception:  # fail loud: a dying handler thread stalls cargo
                    import traceback
                    traceback.print_exc()
                    try:
                        self.send_error(500)
                    except Exception:
                        pass

            def _get(self):
                path = self.path.split("?")[0]
                if path == "/index/config.json":
                    reg.build_index()
                    body = (reg.root / "index" / "config.json").read_bytes()
                elif path.startswith("/crates/") and path.endswith(".crate"):
                    crate = reg.root / "crates" / path.rsplit("/", 1)[1]
                    if not crate.is_file():
                        self.send_error(404)
                        return
                    body = crate.read_bytes()
                elif path.startswith("/index/"):
                    rel = path[len("/index/"):]
                    parts = rel.split("/")
                    if len(parts) != 3 or not all(parts):
                        self.send_error(404)
                        return
                    index_file = reg.root / "index" / rel
                    if not index_file.is_file():
                        self.send_error(404)
                        return
                    body = index_file.read_bytes()
                else:
                    self.send_error(404)
                    return
                self.send_response(200)
                self.send_header("Content-Length", str(len(body)))
                self.send_header("Content-Type", "text/plain")
                self.end_headers()
                self.wfile.write(body)

        ThreadingHTTPServer(("127.0.0.1", self.port), Handler).serve_forever()


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: sparse_registry.py <registry-root> <port>")
    Registry(Path(sys.argv[1]).resolve(), int(sys.argv[2])).serve()


if __name__ == "__main__":
    main()
