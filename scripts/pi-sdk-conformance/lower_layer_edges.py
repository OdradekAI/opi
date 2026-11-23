"""Measure production references from opi-ai's lower owners."""

from collections import Counter
import json
from pathlib import Path

from rust_source import production_files, production_tokens, rust_module, source_references, tokens


SRC = Path("crates/opi-ai/src")
BASELINE = Path("scripts/pi-sdk-conformance/lower_layer_baseline.json")
UPPER = {"api", "providers", "models"}
EXECUTION_OWNERS = {"stream", "execution"}
COPILOT_POLICY_SYMBOLS = {
    "GITHUB_COPILOT_MANAGED_HEADERS",
    "github_copilot_route_headers",
    "github_copilot_initiator",
}
TARGETS = {
    "provider": UPPER | {"stream", "transcript", "execution"},
    "stream": UPPER | {"http", "transport", "auth", "catalog", "execution", "transcript"},
    "execution": UPPER | {"http", "transport", "auth", "catalog", "stream", "transcript"},
    "telemetry": UPPER | {"provider", "stream", "execution"},
    "http": UPPER | {"stream", "execution", "transcript"},
    "transcript": UPPER | {"provider", "stream", "execution"},
    "transport": UPPER | {"stream", "execution", "transcript"},
    "auth": UPPER | {"stream", "execution"},
}
LIMITS = (
    "Scoped Rust source references, including named crate-root re-exports",
    "Feature gates are reviewed in the baseline; paths possible in any production configuration are counted",
    "No compiler name resolution, transitive re-export or macro expansion, or feature satisfiability proof",
    "Method ownership and protocol decisions expressed in values or branches require a call-chain review",
)


def facade_exports(src):
    lib = src / "lib.rs"
    if not lib.is_file():
        return {}
    return {
        reference.alias or reference.path[-1]: (
            reference.path if reference.path[0] == "crate" else ("crate", *reference.path)
        )
        for reference in source_references(lib.read_text(encoding="utf-8"), ("crate",))
        if reference.top_level and reference.kind == "pub use" and reference.path[-1] != "*"
    }


def guarded_sites(text, file, exports, *, production=True):
    module = rust_module(file)
    owner = module[1]
    for reference in source_references(text, module, production=production):
        original = reference.path
        path = original
        if path[:1] == ("crate",) and len(path) > 1:
            path = exports.get(path[1], path[:2]) + path[2:]
        route_context = owner in EXECUTION_OWNERS and path[:3] == ("crate", "provider", "routes")
        guarded = len(path) > 1 and path[0] == "crate" and path[1] in TARGETS[owner]
        if reference.crate_namespace or guarded or route_context:
            yield {
                "file": (SRC / file).as_posix(),
                "line": reference.line,
                "from": owner,
                "to": path[1] if len(path) > 1 else "*",
                "path": "::".join(path),
                "written_path": "::".join(original),
                "kind": reference.kind,
            }


def provider_copilot_policy_sites(text, file):
    words, lines = production_tokens(*tokens(text))
    return [
        {"file": (SRC / file).as_posix(), "line": line, "kind": kind, "symbol": name}
        for kind, name, line in zip(words, words[1:], lines[1:])
        if kind in {"const", "fn"} and name in COPILOT_POLICY_SYMBOLS
    ]


def lower_layer_report(repo):
    src = (repo / SRC).resolve()
    exports = facade_exports(src)
    entrypoints = [src / (name + ".rs") for name in TARGETS]
    files = {}
    for entry in entrypoints:
        for path in [entry, *entry.with_suffix("").rglob("*.rs")]:
            if path.is_file():
                files[path] = path.read_text(encoding="utf-8")
    live = production_files(files, entrypoints=entrypoints)

    allowances = {}
    for group in json.loads((repo / BASELINE).read_text(encoding="utf-8")):
        if group["role"] not in {"contract", "exception"}:
            raise ValueError(f"unknown lower-layer baseline role: {group['role']}")
        for kind, path, sites in group["sites"]:
            key = ((SRC / group["file"]).as_posix(), kind, path)
            if key in allowances or sites < 1:
                raise ValueError(f"duplicate or empty lower-layer baseline entry: {key}")
            allowances[key] = (sites, group)

    report = {
        "rules": {
            source: sorted(targets | ({"provider::routes"} if source in EXECUTION_OWNERS else set()))
            for source, targets in TARGETS.items()
        },
        "limits": list(LIMITS),
        "reviewed_contracts": [],
        "known_exceptions": [],
        "new_dependencies": [],
        "unused_baseline": [],
        "excluded_tests": [],
        "provider_copilot_policy": [],
    }
    used = Counter()
    for file, text in sorted(files.items()):
        relative = file.relative_to(src)
        if file in live and rust_module(relative)[1] == "provider":
            report["provider_copilot_policy"].extend(provider_copilot_policy_sites(text, relative))
        sites = list(guarded_sites(text, relative, exports)) if file in live else []
        site_ids = Counter((site["line"], site["kind"], site["path"]) for site in sites)
        for site in guarded_sites(text, relative, exports, production=False):
            identity = site["line"], site["kind"], site["path"]
            if site_ids[identity]:
                site_ids[identity] -= 1
            else:
                report["excluded_tests"].append(site)
        for site in sorted(sites, key=lambda item: (item["line"], item["path"], item["kind"])):
            key = site["file"], site["kind"], site["path"]
            allowed, group = allowances.get(key, (0, None))
            if used[key] >= allowed:
                report["new_dependencies"].append(site)
                continue
            used[key] += 1
            category = "reviewed_contracts" if group["role"] == "contract" else "known_exceptions"
            report[category].append({**site, "feature": group["feature"], "reason": group["reason"]})

    for (file, kind, path), (allowed, _) in sorted(allowances.items()):
        remaining = allowed - used[file, kind, path]
        if remaining:
            report["unused_baseline"].append({
                "file": file, "kind": kind, "path": path, "unused_sites": remaining,
            })
    return report
