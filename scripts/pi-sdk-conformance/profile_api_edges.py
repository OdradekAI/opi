"""Audit lexical dependencies of provider profiles on the API namespace."""
from pathlib import Path

from rust_source import production_files, rust_module, source_references


PROFILES = Path("crates/opi-ai/src/providers")
ACCOUNTING = {
    "OpenRouterCostDetails", "OpenRouterUsageAccounting", "apply_openrouter_reported_cost",
}
CONSTRUCTORS = {
    "system_one::TypesafeSystemOneApi",
    "openai_decisions::OpenAiDecisionsApi",
    "cloudflare_workers_ai_system_one::CloudflareWorkersAiSystemOneApi",
    "openrouter_images::OpenRouterImagesApi",
}

def permission(file, path, kind, alias, top_level, following, previous):
    tail = "::".join(path[2:])
    if (file == "openrouter.rs" and kind == "pub use" and top_level and alias is None
            and path[2:4] == ("shared", "openrouter_usage") and len(path) == 5
            and path[-1] in ACCOUNTING):
        return "public OpenRouter accounting export"
    if file == "collection.rs" and kind == "qualified":
        if tail.endswith("::new") and tail[:-5] in CONSTRUCTORS and following == "(":
            return "construct builtin adapter"
        if tail == "ClassifierApi" and previous == "dyn" and following == "<":
            return "register typed classifier"
    return None


def scan_profile(source: str, file: str) -> list[dict]:
    relative = PROFILES / file
    module = rust_module(Path("providers") / file)
    rows = []
    for reference in source_references(source, module):
        path = reference.path
        # Whole-crate aliases and globs obscure which owners enter the scope.
        if reference.crate_namespace or path[:2] == ("crate", "api"):
            rows.append({
                "file": relative.as_posix(), "line": reference.line, "path": "::".join(path),
                "kind": reference.kind,
                "permission": permission(file, path, reference.kind, reference.alias, reference.top_level,
                                         reference.following, reference.previous),
            })
    return sorted(rows, key=lambda row: (row["line"], row["path"]))


def profile_api_edges(repo: Path) -> list[dict]:
    directory = (repo / PROFILES).resolve()
    files = {path: path.read_text(encoding="utf-8") for path in directory.rglob("*.rs")}
    return [row for file in sorted(production_files(files, entrypoints=[directory / "mod.rs"]))
            for row in scan_profile(files[file], file.relative_to(directory).as_posix())]
