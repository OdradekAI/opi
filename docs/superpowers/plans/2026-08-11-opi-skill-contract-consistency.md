# Opi Skill Contract Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: use test-driven development and
> verification-before-completion. Execute inline unless the user explicitly
> requests subagents. Repository rules override generic commit guidance: leave
> all changes uncommitted unless the user separately authorizes a commit.

**Goal:** Extend the existing documentation checker so project-local Opi skill
frontmatter, Codex sidecars, and English/Chinese indexes cannot drift
mechanically.

**Architecture:** Add narrow, dependency-free parsers and a
`check_skill_contracts()` orchestration function to `scripts/opi-doc-check.py`.
Exercise them through isolated `unittest` fixtures, then wire the selected
skill/index paths into the existing local-link pass. Correct only the two known
sidecar wording mismatches and the documentation that enumerates checker scope.

**Tech Stack:** Python 3 standard library (`pathlib`, `re`, `tempfile`,
`unittest`, `importlib`), Markdown, YAML-like scalar parsing.

---

## File map

- Create `scripts/test_opi_doc_check.py`: isolated unit tests for skill
  discovery, metadata, sidecar policy, and index equality.
- Modify `scripts/opi-doc-check.py`: pure parsing helpers, skill-contract
  validation, and main/link-check integration.
- Modify `.claude/skills/opi-audit/agents/openai.yaml`: replace the stale
  implementation-range wording with current committed `HEAD`.
- Modify `.claude/skills/opi-eval/agents/openai.yaml`: describe case/model
  evaluation rather than a phase.
- Modify `.claude/skills/opi-document/SKILL.md`: include skill-contract checks
  in the documented `opi-doc-check.py` scope.
- Modify
  `.claude/skills/opi-document/references/documentation-checks.md`: add the
  same source/check ownership row.

No Rust, Cargo, canonical ledger, product spec, changelog, or release-state
file changes.

### Task 1: Add the isolated test harness and valid-contract behavior

**Files:**

- Create: `scripts/test_opi_doc_check.py`
- Modify: `scripts/opi-doc-check.py`

- [ ] **Step 1: Write a failing valid-contract test**

Create the test module with an isolated module load and fixture helpers:

```python
from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("opi-doc-check.py")
SPEC = importlib.util.spec_from_file_location("opi_doc_check", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
doc_check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(doc_check)


class SkillContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.original_root = doc_check.ROOT
        self.original_errors = doc_check.ERRORS
        doc_check.ROOT = self.root
        doc_check.ERRORS = []

    def tearDown(self) -> None:
        doc_check.ROOT = self.original_root
        doc_check.ERRORS = self.original_errors
        self.temp.cleanup()

    def write(self, rel: str, text: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")

    def write_indexes(self, names: tuple[str, ...]) -> None:
        rows = "\n".join(f"| `{name}` | contract |" for name in names)
        table = f"| Skill | Contract |\n|---|---|\n{rows}\n"
        self.write(".claude/skills/README.md", table)
        self.write(".claude/skills/README.zh.md", table)

    def write_skill(
        self,
        name: str = "opi-example",
        *,
        entry: str = "SKILL.md",
        frontmatter_name: str | None = None,
        disable_model_invocation: str = "true",
        prompt_skill: str | None = None,
        allow_implicit_invocation: str = "false",
    ) -> None:
        self.write(
            f".claude/skills/{name}/{entry}",
            "---\n"
            f"name: {frontmatter_name or name}\n"
            f"disable-model-invocation: {disable_model_invocation}\n"
            "description: Test skill.\n"
            "---\n\n"
            "# Test Skill\n",
        )
        self.write(
            f".claude/skills/{name}/agents/openai.yaml",
            "interface:\n"
            "  display_name: \"Test Skill\"\n"
            "  short_description: \"Test contract\"\n"
            f"  default_prompt: \"Use ${prompt_skill or name} for this task.\"\n"
            "policy:\n"
            f"  allow_implicit_invocation: {allow_implicit_invocation}\n",
        )

    def test_matching_skill_contract_passes(self) -> None:
        self.write_skill()
        self.write_indexes(("opi-example",))

        checker = getattr(doc_check, "check_skill_contracts", None)
        self.assertIsNotNone(checker, "skill contract checker must exist")
        paths = checker()

        self.assertEqual([], doc_check.ERRORS)
        self.assertEqual(
            {
                ".claude/skills/README.md",
                ".claude/skills/README.zh.md",
                ".claude/skills/opi-example/SKILL.md",
            },
            set(paths),
        )


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run the test and verify RED**

Run:

```text
python -m unittest scripts/test_opi_doc_check.py -v
```

Expected: one assertion failure stating `skill contract checker must exist`.

- [ ] **Step 3: Implement the narrow parsers and valid-contract path**

Add these helpers before `LINK_RE` in `scripts/opi-doc-check.py`:

```python
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
    if not skills_root.is_dir():
        ERRORS.append("missing directory: .claude/skills")
        return index_paths

    for directory in sorted(skills_root.glob("opi-*")):
        if not directory.is_dir():
            continue
        name = directory.name
        candidates = [
            path
            for path in directory.iterdir()
            if path.is_file() and path.name.lower() == "skill.md"
        ]
        if len(candidates) != 1:
            ERRORS.append(
                f".claude/skills/{name}: expected exactly one SKILL.md or skill.md"
            )
            continue

        skill_rel = candidates[0].relative_to(ROOT).as_posix()
        selected_paths.append(skill_rel)

    return [*index_paths, *selected_paths]
```

- [ ] **Step 4: Run the test and verify GREEN**

Run the same unittest command. Expected: `1 test` and `OK`.

### Task 2: Add frontmatter and entry-file failure coverage

**Files:**

- Modify: `scripts/test_opi_doc_check.py`
- Modify: `scripts/opi-doc-check.py` only if a test exposes a defect

- [ ] **Step 1: Add one failing frontmatter-name test**

```python
    def test_frontmatter_name_must_match_directory(self) -> None:
        self.write_skill(frontmatter_name="opi-other")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/SKILL.md: frontmatter name must equal 'opi-example'",
            doc_check.ERRORS,
        )
```

Run the focused test and require an assertion failure before retaining or
adjusting implementation:

```text
python -m unittest scripts.test_opi_doc_check.SkillContractTests.test_frontmatter_name_must_match_directory -v
```

Expected after implementation: `OK`.

- [ ] **Step 2: Repeat red/green for explicit Claude invocation metadata**

```python
    def test_claude_invocation_must_be_explicit(self) -> None:
        self.write_skill(disable_model_invocation="false")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/SKILL.md: disable-model-invocation must be true",
            doc_check.ERRORS,
        )
```

After each test has failed for its expected missing validation, add the
frontmatter validation immediately after `selected_paths.append(skill_rel)`:

```python
        metadata = frontmatter_scalars(skill_rel)
        if metadata.get("name") != name:
            ERRORS.append(f"{skill_rel}: frontmatter name must equal {name!r}")
        if metadata.get("disable-model-invocation") != "true":
            ERRORS.append(f"{skill_rel}: disable-model-invocation must be true")
```

- [ ] **Step 3: Repeat red/green for lowercase entry compatibility**

```python
    def test_lowercase_skill_entry_is_supported(self) -> None:
        self.write_skill(entry="skill.md")
        self.write_indexes(("opi-example",))

        paths = doc_check.check_skill_contracts()

        self.assertEqual([], doc_check.ERRORS)
        self.assertIn(".claude/skills/opi-example/skill.md", paths)
```

Run each focused test after writing it, then run the whole test module. Do not
change production code if the intended behavior already passes; record that as
existing coverage rather than manufacturing a red result.

### Task 3: Add sidecar-policy failure coverage

**Files:**

- Modify: `scripts/test_opi_doc_check.py`
- Modify: `scripts/opi-doc-check.py` only as required by a failing test

- [ ] **Step 1: Add and run the implicit-invocation test**

```python
    def test_codex_invocation_must_be_explicit(self) -> None:
        self.write_skill(allow_implicit_invocation="true")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/agents/openai.yaml: "
            "policy.allow_implicit_invocation must be false",
            doc_check.ERRORS,
        )
```

After the first sidecar test fails, add this block after the frontmatter
validation. Run it green before adding the second test; extend the same block
only after the second test fails:

```python
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
```

- [ ] **Step 2: Add and run the prompt-skill identity test**

```python
    def test_default_prompt_must_name_owning_skill(self) -> None:
        self.write_skill(prompt_skill="opi-other")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/agents/openai.yaml: "
            "default_prompt must invoke $opi-example",
            doc_check.ERRORS,
        )
```

For each test, first run the focused command and confirm failure for the
expected missing behavior, then make the smallest parser/check adjustment and
rerun to `OK`.

### Task 4: Add EN/ZH index equality coverage

**Files:**

- Modify: `scripts/test_opi_doc_check.py`
- Modify: `scripts/opi-doc-check.py` only as required

- [ ] **Step 1: Add a helper for writing different index sets**

Replace `write_indexes` with a small `write_index` primitive plus the existing
wrapper:

```python
    def write_index(self, rel: str, names: tuple[str, ...]) -> None:
        rows = "\n".join(f"| `{name}` | contract |" for name in names)
        table = f"| Skill | Contract |\n|---|---|\n{rows}\n"
        self.write(rel, table)

    def write_indexes(self, names: tuple[str, ...]) -> None:
        self.write_index(".claude/skills/README.md", names)
        self.write_index(".claude/skills/README.zh.md", names)
```

- [ ] **Step 2: Add and run the missing-index-entry test**

```python
    def test_each_index_must_match_discovered_skills(self) -> None:
        self.write_skill()
        self.write_index(".claude/skills/README.md", ("opi-example",))
        self.write_index(".claude/skills/README.zh.md", ())

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/README.zh.md: skill index differs; "
            "missing=['opi-example'], extra=[]",
            doc_check.ERRORS,
        )
```

Run focused red/green, then the whole Python test module.

After the test fails, add `discovered: set[str] = set()` beside
`selected_paths`, add `discovered.add(name)` after each directory name is
resolved, and add this comparison before the return:

```python
    for index_rel in index_paths:
        indexed = skill_index_names(index_rel)
        if indexed != discovered:
            missing = sorted(discovered - indexed)
            extra = sorted(indexed - discovered)
            ERRORS.append(
                f"{index_rel}: skill index differs; "
                f"missing={missing!r}, extra={extra!r}"
            )
```

### Task 5: Integrate the check and correct known metadata drift

**Files:**

- Modify: `scripts/opi-doc-check.py`
- Modify: `.claude/skills/opi-audit/agents/openai.yaml`
- Modify: `.claude/skills/opi-eval/agents/openai.yaml`
- Modify: `.claude/skills/opi-document/SKILL.md`
- Modify:
  `.claude/skills/opi-document/references/documentation-checks.md`

- [ ] **Step 1: Wire skill paths into `main()`**

Change the beginning and link loop in `main()` to:

```python
def main() -> int:
    docs = check_counterparts()
    skill_docs = check_skill_contracts()
    check_root_guidance_lockstep()
    check_workspace_graph()
    phase15_safety_sandbox_docs()
    phase16_command_execution_docs()
    check_top_level_spec()
    check_current_contracts(docs)
    for rel in [*docs, *skill_docs, "AGENTS.md", "CLAUDE.md"]:
        check_local_links(rel)
```

- [ ] **Step 2: Run the repository checker before sidecar fixes**

Run:

```text
python scripts/opi-doc-check.py
```

Expected: structural checks pass. The checker deliberately does not claim to
detect the two free-form semantic mismatches; those are corrected from the
approved design and verified by review.

- [ ] **Step 3: Correct the audit sidecar prompt**

Set its default prompt to:

```yaml
  default_prompt: "Use $opi-audit to independently audit phase=<N> against its registered requirements and the complete relevant implementation at current committed HEAD."
```

- [ ] **Step 4: Correct the eval sidecar prompt**

Set its default prompt to:

```yaml
  default_prompt: "Use $opi-eval to run isolated runtime regression cases against the selected model and preserve normalized findings."
```

- [ ] **Step 5: Update checker-scope documentation surgically**

In `opi-document/SKILL.md`, extend the sentence describing the Python check to
include project-local skill frontmatter, Codex sidecars, and EN/ZH skill-index
membership.

In `documentation-checks.md`, add one row:

```markdown
| Project-local skill names, explicit invocation metadata, Codex sidecars, and EN/ZH skill-index membership | `.claude/skills/opi-*/SKILL.md` or `skill.md`, `agents/openai.yaml`, and the two skill indexes | `python scripts/opi-doc-check.py` |
```

- [ ] **Step 6: Run the exact verification set**

```text
python -m unittest scripts/test_opi_doc_check.py -v
python scripts/opi-doc-check.py
git diff --check
```

Expected:

- all unit tests report `OK`;
- documentation contracts report `PASS`;
- `git diff --check` exits 0 without output.

- [ ] **Step 7: Review scope and leave changes uncommitted**

Run:

```text
git status --short
git diff -- scripts/opi-doc-check.py scripts/test_opi_doc_check.py .claude/skills/opi-audit/agents/openai.yaml .claude/skills/opi-eval/agents/openai.yaml .claude/skills/opi-document/SKILL.md .claude/skills/opi-document/references/documentation-checks.md
```

Verify that every changed line traces to this design, the pre-existing
`docs/research/opi-knowledge-sdk-learning-worker-spec.zh.md` remains untouched,
and no file is staged or committed.

## Plan self-review

- Design scope is covered by Tasks 1-5.
- No generated manifest, second command, external dependency, Rust build, or
  semantic prompt inference was introduced.
- The parser accepts the repository's existing uppercase and lowercase skill
  entry filenames.
- Tests use temporary roots and restore module globals to avoid contaminating
  the real repository.
- The plan does not authorize commits or subagent use.
