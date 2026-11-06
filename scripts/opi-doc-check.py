#!/usr/bin/env python3
"""Check documentation files, local links, metadata and source-derived values.

Prose meaning and runtime behavior belong to review and owning behavior tests.
This standard-library-only check does not compile Rust or require fixed prose.
"""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


CURRENT_CONTEXT_DOCS = (
    "docs/README.md",
    "docs/README.zh.md",
    "docs/GLOSSARY.md",
    "docs/agents/context.md",
    "docs/agents/domain.md",
    "docs/agents/issue-tracker.md",
    "docs/engineering/README.md",
    "docs/iterations/README.md",
    "docs/adr/README.md",
    ".agents/skills/_shared/references/legacy-phase-workflows.md",
)
CONTEXT_ROOTS = ("docs/agents", "docs/engineering", "docs/iterations", "docs/adr")


def read(rel: str) -> str:
    path = ROOT / rel
    if not path.is_file():
        ERRORS.append(f"missing file: {rel}")
        return ""
    data = path.read_bytes()
    try:
        text = data.decode("utf-8-sig")
    except UnicodeDecodeError as exc:
        ERRORS.append(f"not UTF-8: {rel}: {exc}")
        return ""
    if not text.strip():
        ERRORS.append(f"empty document: {rel}")
    return text


def require(rel: str, token: str, *, label: str | None = None) -> None:
    if token not in read(rel):
        ERRORS.append(f"{rel}: missing {label or token!r}")


def read_toml(rel: str) -> dict[str, object]:
    try:
        return tomllib.loads(read(rel))
    except tomllib.TOMLDecodeError as exc:
        ERRORS.append(f"{rel}: invalid TOML: {exc}")
        return {}


def workspace_version() -> str:
    workspace = read_toml("Cargo.toml").get("workspace", {})
    package = workspace.get("package", {}) if isinstance(workspace, dict) else {}
    version = package.get("version") if isinstance(package, dict) else None
    if not isinstance(version, str) or not version:
        ERRORS.append("Cargo.toml: cannot resolve [workspace.package] version")
        return ""
    return version


def rust_u32(rel: str, name: str) -> int | None:
    source = read(rel)
    match = re.search(rf"\b{name}\s*:\s*u32\s*=\s*(\d+)\s*;", source)
    if not match:
        ERRORS.append(f"{rel}: cannot resolve {name}")
        return None
    return int(match.group(1))


def crate_packages() -> list[tuple[str, str]]:
    packages: list[tuple[str, str]] = []
    for manifest_path in sorted((ROOT / "crates").glob("*/Cargo.toml")):
        crate_dir = manifest_path.parent.relative_to(ROOT).as_posix()
        package = read_toml(f"{crate_dir}/Cargo.toml").get("package", {})
        name = package.get("name") if isinstance(package, dict) else None
        if not isinstance(name, str) or not name:
            ERRORS.append(f"{crate_dir}/Cargo.toml: missing package.name")
            continue
        packages.append((name, crate_dir))
    return packages


def check_counterparts() -> list[str]:
    pairs = [
        ("README.md", "README.zh.md"),
    ]
    pairs.extend(
        (f"{crate_dir}/README.md", f"{crate_dir}/README.zh.md")
        for _, crate_dir in crate_packages()
    )
    for english, chinese in pairs:
        read(english)
        read(chinese)
    return [path for pair in pairs for path in pair]


def check_root_guidance_lockstep() -> None:
    agents = read("AGENTS.md")
    claude = read("CLAUDE.md")
    if claude.strip() == "AGENTS.md":
        # Degraded clone: the filesystem materialized the CLAUDE.md symlink as
        # a text file holding its target path. Guidance stays single-sourced;
        # restore the symlink on such a checkout before committing.
        print(
            "WARN: CLAUDE.md is a materialized symlink placeholder, not a symlink",
            file=sys.stderr,
        )
        return
    if re.sub(r"\s+", " ", agents).strip() != re.sub(r"\s+", " ", claude).strip():
        ERRORS.append(
            "AGENTS.md and CLAUDE.md diverge; CLAUDE.md is a symlink to AGENTS.md,"
            " so edit AGENTS.md and restore the link"
        )


def scalar(value: str) -> str:
    value = value.strip()
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
        return value[1:-1]
    return value


def frontmatter_scalars(rel: str) -> dict[str, str]:
    lines = read(rel).splitlines()
    if not lines or lines[0] != "---":
        ERRORS.append(f"{rel}: missing YAML frontmatter")
        return {}
    try:
        end = lines.index("---", 1)
    except ValueError:
        ERRORS.append(f"{rel}: unterminated YAML frontmatter")
        return {}
    values: dict[str, str] = {}
    for line in lines[1:end]:
        if not line or line[0].isspace():
            continue
        match = re.fullmatch(r"([A-Za-z0-9_-]+):\s*(.*)", line)
        if match:
            values[match.group(1)] = scalar(match.group(2))
    return values


def sidecar_scalars(rel: str) -> dict[str, str]:
    values: dict[str, str] = {}
    section: str | None = None
    for line in read(rel).splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        top = re.fullmatch(r"([A-Za-z0-9_-]+):\s*", line)
        if top:
            section = top.group(1)
            continue
        nested = re.fullmatch(r"  ([A-Za-z0-9_-]+):\s*(.*)", line)
        if section is not None and nested:
            values[f"{section}.{nested.group(1)}"] = scalar(nested.group(2))
    return values


def skill_index_names(rel: str) -> set[str]:
    return set(
        re.findall(r"(?m)^\|\s*`(opi-[a-z0-9-]+)`\s*\|", read(rel))
    )


def check_skill_contracts() -> list[str]:
    skills_root = ROOT / ".claude" / "skills"
    index_paths = [
        ".claude/skills/README.md",
        ".claude/skills/README.zh.md",
    ]
    selected_paths: list[str] = []
    discovered: set[str] = set()
    if not skills_root.is_dir():
        ERRORS.append("missing directory: .claude/skills")
        return index_paths

    for directory in sorted(skills_root.glob("opi-*")):
        if not directory.is_dir():
            continue
        name = directory.name
        discovered.add(name)
        candidates = [
            path
            for path in directory.iterdir()
            if path.is_file() and path.name == "SKILL.md"
        ]
        if len(candidates) != 1:
            ERRORS.append(
                f".claude/skills/{name}: expected exactly one SKILL.md"
            )
            continue

        skill_rel = candidates[0].relative_to(ROOT).as_posix()
        selected_paths.append(skill_rel)
        metadata = frontmatter_scalars(skill_rel)
        if metadata.get("name") != name:
            ERRORS.append(f"{skill_rel}: frontmatter name must equal {name!r}")
        if metadata.get("disable-model-invocation") != "true":
            ERRORS.append(f"{skill_rel}: disable-model-invocation must be true")

        sidecar_rel = f".claude/skills/{name}/agents/openai.yaml"
        sidecar = sidecar_scalars(sidecar_rel)
        for field in (
            "interface.display_name",
            "interface.short_description",
            "interface.default_prompt",
        ):
            if not sidecar.get(field):
                ERRORS.append(f"{sidecar_rel}: missing non-empty {field}")
        if f"${name}" not in sidecar.get("interface.default_prompt", ""):
            ERRORS.append(f"{sidecar_rel}: default_prompt must invoke ${name}")
        if sidecar.get("policy.allow_implicit_invocation") != "false":
            ERRORS.append(
                f"{sidecar_rel}: policy.allow_implicit_invocation must be false"
            )

    for index_rel in index_paths:
        indexed = skill_index_names(index_rel)
        if indexed != discovered:
            missing = sorted(discovered - indexed)
            extra = sorted(indexed - discovered)
            ERRORS.append(
                f"{index_rel}: skill index differs; "
                f"missing={missing!r}, extra={extra!r}"
            )

    # Include references without pinning their prose or a fixed reference list.
    # A renamed or new reference receives the same local-link validation.
    selected_paths.extend(
        path.relative_to(ROOT).as_posix()
        for path in skills_root.rglob("*.md")
        if path.name != "skill.md"
    )
    return sorted(set([*index_paths, *selected_paths]))


LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")


def check_context_docs() -> None:
    """Check current context entry points, links and the single glossary owner."""
    paths = set(CURRENT_CONTEXT_DOCS)
    for root in CONTEXT_ROOTS:
        paths.update(
            path.relative_to(ROOT).as_posix()
            for path in (ROOT / root).rglob("*.md")
        )
    for rel in sorted(paths):
        check_local_links(rel)
    # A project adapter points consumers at docs/GLOSSARY.md. Catch accidental
    # default-path creation instead of maintaining two competing vocabularies.
    for rel in ("GLOSSARY.md", "GLOSSARY-MAP.md"):
        if (ROOT / rel).exists():
            ERRORS.append(f"{rel}: duplicate domain entry; use docs/GLOSSARY.md")


def check_local_links(rel: str) -> None:
    text = read(rel)
    base = (ROOT / rel).parent
    for raw in LINK_RE.findall(text):
        raw = raw.strip()
        if raw.startswith("<") and ">" in raw:
            target = raw[1 : raw.index(">")]
        else:
            target = raw.split(maxsplit=1)[0]
        if target.startswith(("#", "http://", "https://", "mailto:")):
            continue
        target = unquote(target.split("#", 1)[0])
        if not target or target.startswith("/") or any(c in target for c in "{}*<>"):
            continue
        # `.repo/` is the gitignored, non-normative local evidence cache: it is
        # not part of a fresh checkout, so links into it are exempt from
        # existence checks (a checked-out tree would always report them broken).
        try:
            (base / target).resolve().relative_to(ROOT / ".repo")
            continue
        except ValueError:
            pass
        if not (base / target).resolve().exists():
            ERRORS.append(f"{rel}: broken local link: {target}")


def check_current_contracts(doc_paths: list[str]) -> None:
    # Check the declared values, not the surrounding English/Chinese wording.
    # Presence is not proof that every prose claim about a version is correct.
    version = workspace_version()
    if version:
        for rel in doc_paths:
            require(rel, f"`{version}`", label=f"current workspace version `{version}`")

    constants = [
        (
            "NDJSON_SCHEMA_VERSION",
            rust_u32("crates/opi-coding-agent/src/runner.rs", "NDJSON_SCHEMA_VERSION"),
        ),
        ("SDK_SCHEMA_VERSION", rust_u32("crates/opi-agent/src/sdk.rs", "SDK_SCHEMA_VERSION")),
    ]
    for name, value in constants:
        if value is None:
            continue
        for rel in ("README.md", "README.zh.md"):
            values = {
                int(match)
                for match in re.findall(rf"\b{name}\s*=\s*([0-9]+)(?!\w|\.[0-9])", read(rel))
            }
            if values != {value}:
                ERRORS.append(f"{rel}: expected {name} = {value}; found {sorted(values)}")


def main() -> int:
    docs = check_counterparts()
    skill_docs = check_skill_contracts()
    check_root_guidance_lockstep()
    check_context_docs()
    check_current_contracts(docs)
    for rel in [*docs, *skill_docs, "AGENTS.md", "CLAUDE.md"]:
        check_local_links(rel)

    if ERRORS:
        for error in sorted(set(ERRORS)):
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("opi documentation contracts: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
