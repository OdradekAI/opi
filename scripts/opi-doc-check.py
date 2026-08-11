#!/usr/bin/env python3
"""Fast documentation contract checks that do not compile Rust test binaries."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path
from urllib.parse import unquote


ROOT = Path(__file__).resolve().parent.parent
ERRORS: list[str] = []


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


def require_tokens(rel: str, label: str, tokens: tuple[str, ...]) -> None:
    text = read(rel)
    missing = [token for token in tokens if token not in text]
    if missing:
        ERRORS.append(f"{rel}: {label} missing semantic tokens {missing!r}")


def workspace_version() -> str:
    cargo = read("Cargo.toml")
    match = re.search(
        r"(?ms)^\[workspace\.package\]\s*$.*?^version\s*=\s*\"([^\"]+)\"",
        cargo,
    )
    if not match:
        ERRORS.append("Cargo.toml: cannot resolve [workspace.package] version")
        return ""
    return match.group(1)


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
        manifest = tomllib.loads(read(f"{crate_dir}/Cargo.toml"))
        name = manifest.get("package", {}).get("name")
        if not isinstance(name, str):
            ERRORS.append(f"{crate_dir}/Cargo.toml: missing package.name")
            continue
        packages.append((name, crate_dir))
    return packages


def check_counterparts() -> list[str]:
    pairs = [
        ("README.md", "README.zh.md"),
        ("docs/opi-spec.md", "docs/opi-spec.zh.md"),
    ]
    pairs.extend(
        (f"{crate_dir}/README.md", f"{crate_dir}/README.zh.md")
        for _, crate_dir in crate_packages()
    )
    for english, chinese in pairs:
        read(english)
        read(chinese)
    return [path for pair in pairs for path in pair]


def normalize_root_guidance(text: str, *, claude: bool) -> str:
    if not claude:
        return text
    replacements = {
        "# CLAUDE.md": "# AGENTS.md",
        "Claude Code (claude.ai/code)": "Codex (Codex.ai/code)",
        "`AGENTS.md` is the Codex-flavored sibling of this file":
            "`CLAUDE.md` is the Claude Code-flavored sibling of this file",
        "Co-Authored-By: Claude": "Co-Authored-By: Codex",
    }
    for old, new in replacements.items():
        text = text.replace(old, new)
    return text


def check_root_guidance_lockstep() -> None:
    agents = normalize_root_guidance(read("AGENTS.md"), claude=False)
    claude = normalize_root_guidance(read("CLAUDE.md"), claude=True)
    if re.sub(r"\s+", " ", agents).strip() != re.sub(r"\s+", " ", claude).strip():
        ERRORS.append(
            "AGENTS.md and CLAUDE.md drift beyond their four intentional flavor differences"
        )


def workspace_graph() -> dict[str, set[str]]:
    root_manifest = tomllib.loads(read("Cargo.toml"))
    members = root_manifest.get("workspace", {}).get("members", [])
    manifests: dict[str, dict] = {}
    for member in members:
        manifest = tomllib.loads(read(f"{member}/Cargo.toml"))
        name = manifest.get("package", {}).get("name")
        if not isinstance(name, str):
            ERRORS.append(f"{member}/Cargo.toml: missing package.name")
            continue
        manifests[name] = manifest

    names = set(manifests)
    graph: dict[str, set[str]] = {}
    for name, manifest in manifests.items():
        deps: set[str] = set()
        for table_name in ("dependencies", "build-dependencies"):
            for dep_name, config in manifest.get(table_name, {}).items():
                package_name = config.get("package", dep_name) if isinstance(config, dict) else dep_name
                if package_name in names:
                    deps.add(package_name)
        graph[name] = deps
    return graph


def graph_from_guidance(rel: str) -> dict[str, set[str]]:
    text = read(rel)
    match = re.search(r"(?ms)^## Workspace layout\s+.*?```text\s*(.*?)```", text)
    if not match:
        ERRORS.append(f"{rel}: missing Workspace layout text block")
        return {}
    graph: dict[str, set[str]] = {}
    for line in match.group(1).splitlines():
        contract = line.split(" - ", 1)[0].strip()
        if not contract.startswith("opi-"):
            continue
        name = contract.split(maxsplit=1)[0]
        if "(no internal deps)" in contract:
            graph[name] = set()
        elif "->" in contract:
            graph[name] = {dep.strip() for dep in contract.split("->", 1)[1].split(",")}
    return graph


def check_workspace_graph() -> None:
    expected = workspace_graph()
    for rel in ("AGENTS.md", "CLAUDE.md"):
        actual = graph_from_guidance(rel)
        if actual != expected:
            ERRORS.append(f"{rel}: workspace dependency graph differs from Cargo manifests")


def phase15_safety_sandbox_docs() -> None:
    """Compatibility check id retained by the normative Phase 15 SC11 row."""
    operations = read("crates/opi-coding-agent/src/tool/operations.rs")
    interactive = read("crates/opi-coding-agent/src/interactive.rs")
    main = read("crates/opi-coding-agent/src/main.rs")
    if "#![forbid(unsafe_code)]" not in operations:
        ERRORS.append("tool/operations.rs: missing forbid(unsafe_code) safety boundary")
    if '"/trust"' in interactive:
        ERRORS.append("interactive.rs: built-in /trust mutation command is not allowed")
    if "ProjectTrustResolverRegistry::new()" not in main:
        ERRORS.append("main.rs: standard CLI no longer constructs the trust resolver registry")
    if "registry.register(" in main:
        ERRORS.append("main.rs: standard CLI must not register a native trust resolver")


def phase16_command_execution_docs() -> None:
    """Stable Phase 16 product and source-derived documentation contracts."""
    lifecycle = (
        "Minimal Runtime",
        "Installed",
        "Trusted",
        "Enabled",
        "Selected",
        "Permitted",
        "fail-closed",
        "opi-sandbox",
    )
    standalone = (
        "SandboxPolicy",
        "SandboxRequest",
        "SandboxRunner",
        "SandboxEvent",
        "SandboxResult",
        "opi-sandbox backend --stdio",
    )
    migration = ("[sandbox]", "--sandbox", "--sandbox-require")
    for rel in ("docs/opi-spec.md", "docs/opi-spec.zh.md"):
        require_tokens(rel, "Phase 16 lifecycle", lifecycle)
        require_tokens(rel, "standalone sandbox SDK/CLI", standalone)
        require_tokens(rel, "removed core sandbox surface", migration)
        require_tokens(
            rel,
            "Phase 16 non-goals",
            (
                "Docker",
                "VM",
                "SSH",
                "remote" if rel.endswith("opi-spec.md") else "远程",
                "AppContainer",
            ),
        )
        require_tokens(rel, "Windows L0 posture", ("Windows", "L0", "Job Object"))

    for rel in ("AGENTS.md", "CLAUDE.md"):
        require_tokens(rel, "Phase 16 lifecycle", lifecycle)
        require_tokens(rel, "removed core sandbox surface", migration)
        require_tokens(rel, "Phase 16 non-goals", ("Docker", "VM", "SSH"))
        require_tokens(rel, "Windows L0 posture", ("Windows", "L0", "Job-Object"))

    require_tokens(
        "crates/opi-sandbox/src/main.rs",
        "native CLI argv",
        ("std::env::args_os()",),
    )
    require_tokens(
        "crates/opi-sandbox/src/cli.rs",
        "CLI exit mapping",
        (
            "SetupFailureReason::InvalidRequest",
            "SetupFailureReason::UnsupportedPlatform",
            "SandboxOutcome::TimedOut",
            "SandboxOutcome::Cancelled",
            "124",
            "125",
            "130",
        ),
    )
    require_tokens(
        "crates/opi-sandbox/src/platform/mod.rs",
        "native platform dispatch",
        ('target_os = "linux"', 'target_os = "macos"', "windows::posture()"),
    )
    require_tokens(
        "crates/opi-sandbox/src/platform/macos.rs",
        "canonical sandbox-exec probe",
        (
            'const SANDBOX_EXEC_PATH: &str = "/usr/bin/sandbox-exec"',
            "std::process::Command::new(&bin)",
            "SandboxExecStatus::Missing",
            "SandboxExecStatus::Unusable",
        ),
    )


LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
SPEC_ID_RE = re.compile(
    r"\b(?:AUTH|GOAL|PRIN|PLACE|CAP|CTRL|INV|GATE|STRAT|PHASE)-\d{3}\b"
)


def check_top_level_spec() -> None:
    english = read("docs/opi-spec.md")
    chinese = read("docs/opi-spec.zh.md")

    expected_headings = {
        "docs/opi-spec.md": (
            "## 1. Document Authority and Reading Model",
            "## 2. Mission, Goals, and Non-Goals",
            "## 3. First Design Doctrine",
            "## 4. System Placement and Dependency Direction",
            "## 5. Long-Term Capability Ladder",
            "## 6. Cross-Cutting Control Planes",
            "## 7. Durable Architecture Invariants",
            "## 8. Capability Admission and Promotion Gates",
            "## 9. Current Strategic Priorities",
            "## 10. Phase Derivation and Verification",
            "## 11. Authoritative Contracts and Evidence Index",
        ),
        "docs/opi-spec.zh.md": (
            "## 1. 文档权威与阅读模型",
            "## 2. 使命、目标与非目标",
            "## 3. 首要设计准则",
            "## 4. 系统归属与依赖方向",
            "## 5. 长期能力阶梯",
            "## 6. 横切控制平面",
            "## 7. 持久架构不变量",
            "## 8. 能力准入与晋级门禁",
            "## 9. 当前战略优先级",
            "## 10. Phase 推导与验证",
            "## 11. 权威契约与证据索引",
        ),
    }
    for rel, headings in expected_headings.items():
        text = english if rel.endswith("opi-spec.md") else chinese
        actual = tuple(
            line for line in text.splitlines() if re.match(r"^## \d+\. ", line)
        )
        if actual != headings:
            ERRORS.append(f"{rel}: top-level specification chapter structure differs")

    english_ids = SPEC_ID_RE.findall(english)
    chinese_ids = SPEC_ID_RE.findall(chinese)
    if len(english_ids) != 61:
        ERRORS.append("docs/opi-spec.md: expected 61 stable clause identifiers")
    if len(chinese_ids) != 61:
        ERRORS.append("docs/opi-spec.zh.md: expected 61 stable clause identifiers")
    if len(english_ids) != len(set(english_ids)):
        ERRORS.append("docs/opi-spec.md: stable clause identifiers are not unique")
    if len(chinese_ids) != len(set(chinese_ids)):
        ERRORS.append("docs/opi-spec.zh.md: stable clause identifiers are not unique")
    if english_ids != chinese_ids:
        ERRORS.append(
            "docs/opi-spec.md and docs/opi-spec.zh.md: stable clause identifiers differ"
        )

    for rel, text in (
        ("docs/opi-spec.md", english),
        ("docs/opi-spec.zh.md", chinese),
    ):
        for line_number, line in enumerate(text.splitlines(), start=1):
            if "MUST" not in line or not line.startswith("| "):
                continue
            cells = [cell.strip() for cell in line.strip("|").split("|")]
            if len(cells) != 4 or not cells[2] or not cells[3]:
                ERRORS.append(
                    f"{rel}:{line_number}: normative MUST lacks owner or verification"
                )
        if re.search(r"(?im)^#{2,6}\s+(?:phase|第.+阶段)\s*\d+\b", text):
            ERRORS.append(f"{rel}: contains a numbered Phase progress heading")
        if re.search(r"(?i)\b(?:TBD|TODO)\b", text):
            ERRORS.append(f"{rel}: contains an unresolved placeholder")
        if re.search(r"(?im)^#{2,6}\s+.*(?:progress|status|进度|状态)\s*$", text):
            ERRORS.append(f"{rel}: contains a progress or status section")


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
        if not (base / target).resolve().exists():
            ERRORS.append(f"{rel}: broken local link: {target}")


def check_current_contracts(doc_paths: list[str]) -> None:
    version = workspace_version()
    if version:
        require(
            "README.md",
            f"The workspace package version in `Cargo.toml` is `{version}`",
            label="current workspace version sentence",
        )
        for crate, crate_dir in crate_packages():
            require(
                f"{crate_dir}/README.md",
                f"Current crate version: `{version}`",
                label=f"{crate} current version sentence",
            )
        for rel in doc_paths:
            if rel.endswith(".zh.md") and rel != "docs/opi-spec.zh.md":
                require(rel, f"`{version}`", label="current workspace version")

    constants = [
        (
            "NDJSON_SCHEMA_VERSION",
            rust_u32("crates/opi-coding-agent/src/runner.rs", "NDJSON_SCHEMA_VERSION"),
        ),
        ("SDK_SCHEMA_VERSION", rust_u32("crates/opi-agent/src/sdk.rs", "SDK_SCHEMA_VERSION")),
        (
            "TRACE_SCHEMA_VERSION",
            rust_u32("crates/opi-agent/src/trace.rs", "TRACE_SCHEMA_VERSION"),
        ),
    ]
    for name, value in constants:
        if value is None:
            continue
        for rel in ("README.md", "README.zh.md"):
            require(rel, f"{name} = {value}", label=f"current {name}")

    stale_claims = {
        "README.md": ["transport abstraction"],
        "README.zh.md": ["transport 抽象"],
        "docs/opi-spec.md": [
            "current `transport` stub is reserved for Phase 4 RPC/proxy transport",
            "current `transport` stub is reserved for the Phase 4 RPC/proxy transport",
        ],
        "docs/opi-spec.zh.md": [
            "当前的 `transport` 存根保留给第 4 阶段 RPC/proxy 传输",
            "当前的 `transport` 存根保留给第 4 阶段 RPC/proxy transport",
        ],
    }
    for rel, phrases in stale_claims.items():
        text = read(rel)
        for phrase in phrases:
            if phrase in text:
                ERRORS.append(f"{rel}: stale current-product claim: {phrase!r}")


def main() -> int:
    docs = check_counterparts()
    check_root_guidance_lockstep()
    check_workspace_graph()
    phase15_safety_sandbox_docs()
    phase16_command_execution_docs()
    check_top_level_spec()
    check_current_contracts(docs)
    for rel in [*docs, "AGENTS.md", "CLAUDE.md"]:
        check_local_links(rel)

    if ERRORS:
        for error in sorted(set(ERRORS)):
            print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print("opi documentation contracts: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
