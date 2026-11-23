from __future__ import annotations

import contextlib
import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


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

    def write_index(self, rel: str, names: tuple[str, ...]) -> None:
        rows = "\n".join(f"| `{name}` | contract |" for name in names)
        table = f"| Skill | Contract |\n|---|---|\n{rows}\n"
        self.write(rel, table)

    def write_indexes(self, names: tuple[str, ...]) -> None:
        self.write_index(".claude/skills/README.md", names)
        self.write_index(".claude/skills/README.zh.md", names)


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
            '  display_name: "Test Skill"\n'
            '  short_description: "Test contract"\n'
            f'  default_prompt: "Use ${prompt_skill or name} for this task."\n'
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

    def test_frontmatter_name_must_match_directory(self) -> None:
        self.write_skill(frontmatter_name="opi-other")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/SKILL.md: "
            "frontmatter name must equal 'opi-example'",
            doc_check.ERRORS,
        )

    def test_claude_invocation_must_be_explicit(self) -> None:
        self.write_skill(disable_model_invocation="false")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/SKILL.md: "
            "disable-model-invocation must be true",
            doc_check.ERRORS,
        )

    def test_lowercase_skill_entry_is_rejected(self) -> None:
        self.write_skill(entry="skill.md")
        self.write_indexes(("opi-example",))

        paths = doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example: expected exactly one SKILL.md",
            doc_check.ERRORS,
        )
        self.assertNotIn(".claude/skills/opi-example/skill.md", paths)

    def test_codex_invocation_must_be_explicit(self) -> None:
        self.write_skill(allow_implicit_invocation="true")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/agents/openai.yaml: "
            "policy.allow_implicit_invocation must be false",
            doc_check.ERRORS,
        )

    def test_default_prompt_must_name_owning_skill(self) -> None:
        self.write_skill(prompt_skill="opi-other")
        self.write_indexes(("opi-example",))

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/agents/openai.yaml: "
            "default_prompt must invoke $opi-example",
            doc_check.ERRORS,
        )

    def test_sidecar_interface_fields_must_be_non_empty(self) -> None:
        self.write_skill()
        self.write_indexes(("opi-example",))
        self.write(
            ".claude/skills/opi-example/agents/openai.yaml",
            "interface:\n"
            '  display_name: ""\n'
            '  short_description: "Test contract"\n'
            '  default_prompt: "Use $opi-example for this task."\n'
            "policy:\n"
            "  allow_implicit_invocation: false\n",
        )

        doc_check.check_skill_contracts()

        self.assertIn(
            ".claude/skills/opi-example/agents/openai.yaml: "
            "missing non-empty interface.display_name",
            doc_check.ERRORS,
        )

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

    def test_skill_and_shared_reference_links_are_checked(self) -> None:
        self.write_skill()
        self.write_indexes(("opi-example",))
        for rel in (".claude/skills/opi-example/references/guide.md", ".claude/skills/_shared/references/guide.md"):
            self.write(rel, "[Required source](missing.md)\n")
        paths = doc_check.check_skill_contracts()
        for rel in paths:
            doc_check.check_local_links(rel)
        self.assertEqual(2, len(doc_check.ERRORS), doc_check.ERRORS)
        self.assertTrue(all("broken local link: missing.md" in error for error in doc_check.ERRORS))


class ContextLifecycleTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.root_patch = patch.object(doc_check, "ROOT", self.root)
        self.errors_patch = patch.object(doc_check, "ERRORS", [])
        self.root_patch.start()
        self.errors_patch.start()
        self.addCleanup(self.root_patch.stop)
        self.addCleanup(self.errors_patch.stop)
        for rel in doc_check.CURRENT_CONTEXT_DOCS:
            self.write(rel, "# Current context\n")

    def write(self, rel: str, content: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def test_current_context_passes_without_retired_documents(self) -> None:
        for rel in ("docs/CONTEXT.md", "docs/opi-spec.md", "docs/opi-spec.zh.md"):
            self.assertFalse((self.root / rel).exists())
        doc_check.check_context_docs()
        self.assertEqual([], doc_check.ERRORS)

    def test_current_entry_documents_are_required(self) -> None:
        for rel in doc_check.CURRENT_CONTEXT_DOCS:
            with self.subTest(rel=rel):
                doc_check.ERRORS.clear()
                (self.root / rel).unlink()
                doc_check.check_context_docs()
                self.assertIn(f"missing file: {rel}", doc_check.ERRORS)
                self.write(rel, "# Current context\n")

    def test_new_documents_in_each_context_area_have_links_checked(self) -> None:
        for root in doc_check.CONTEXT_ROOTS:
            with self.subTest(root=root):
                doc_check.ERRORS.clear()
                rel = f"{root}/example/decision.md"
                self.write(rel, "[Required source](missing.md)\n")
                doc_check.check_context_docs()
                self.assertIn(f"{rel}: broken local link: missing.md", doc_check.ERRORS)
                self.write(rel, "# Decision\n")

    def test_current_link_to_removed_glossary_is_rejected(self) -> None:
        self.write("docs/agents/domain.md", "[Glossary](../CONTEXT.md)\n")
        doc_check.check_context_docs()
        self.assertEqual(
            ["docs/agents/domain.md: broken local link: ../CONTEXT.md"],
            doc_check.ERRORS,
        )

    def test_duplicate_root_glossary_is_rejected(self) -> None:
        for rel in ("GLOSSARY.md", "GLOSSARY-MAP.md"):
            with self.subTest(rel=rel):
                doc_check.ERRORS.clear()
                self.write(rel, "# Duplicate default-path vocabulary\n")
                doc_check.check_context_docs()
                self.assertIn(f"{rel}: duplicate domain entry; use docs/GLOSSARY.md", doc_check.ERRORS)
                (self.root / rel).unlink()

    def test_sdk_guidance_checks_current_source_and_guides_only(self) -> None:
        self.write(
            "crates/opi-ai/src/replay.rs",
            "//! Replay contract.\n//! Pinned TypeScript reference: .repo/pi-1.3.0/source.ts\n",
        )
        self.write("crates/opi-ai/docs/provider-auth.zh.md", "Pi 1.3.0 profile table\n")
        self.write("crates/opi-ai/src/tls/mod.rs", "//! The PinnedNodeDefault trust profile.\n")
        self.write(
            "docs/iterations/opi-ai-alignment/evidence/source.md",
            "Pinned TypeScript reference: .repo/pi-1.3.0/packages/ai/src/api/replay.ts\n",
        )
        doc_check.check_opi_ai_current_guidance()
        self.assertEqual(2, len(doc_check.ERRORS))
        self.assertTrue(any(error.startswith("crates/opi-ai/src/replay.rs:2:") for error in doc_check.ERRORS))
        self.assertTrue(any(error.startswith("crates/opi-ai/docs/provider-auth.zh.md:1:") for error in doc_check.ERRORS))


class ContextCliTests(unittest.TestCase):
    def test_main_returns_failure_for_broken_current_context_link(self) -> None:
        # Run the real CLI aggregation without mutating this shared checkout.
        script = rf"""
import importlib.util
import sys
spec = importlib.util.spec_from_file_location("doc_check", {str(SCRIPT)!r})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
original = module.read
module.read = lambda rel: original(rel) + (
    "\n[Broken](missing-current-contract.md)\n"
    if rel == "docs/engineering/README.md" else ""
)
sys.exit(module.main())
"""
        result = subprocess.run([sys.executable, "-c", script], capture_output=True, text=True, check=False)
        self.assertEqual(1, result.returncode, result.stderr)
        self.assertIn("docs/engineering/README.md: broken local link: missing-current-contract.md", result.stderr)

    def test_main_returns_failure_for_a_stale_sdk_source_reference(self) -> None:
        script = rf"""
import importlib.util
import sys
spec = importlib.util.spec_from_file_location("doc_check", {str(SCRIPT)!r})
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
original = module.read
module.read = lambda rel: original(rel) + (
    "\n//! Pinned TypeScript reference: .repo/pi-1.3.0/packages/ai/src/api/transform-messages.ts\n"
    if rel == "crates/opi-ai/src/replay.rs" else ""
)
sys.exit(module.main())
"""
        result = subprocess.run([sys.executable, "-c", script], capture_output=True, text=True, check=False)
        self.assertEqual(1, result.returncode, result.stderr)
        self.assertIn("FAIL: crates/opi-ai/src/replay.rs:", result.stderr)
        self.assertIn("stale Pi source guidance", result.stderr)


class DocumentFactsTests(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        for name, value in (("ROOT", self.root), ("ERRORS", [])):
            patcher = patch.object(doc_check, name, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.write("Cargo.toml", '[workspace.package]\nversion = "1.2.3"\n')
        self.write("crates/example/Cargo.toml", '[package]\nname = "example"\nversion.workspace = true\n')
        self.docs = ["README.md", "README.zh.md", "crates/example/README.md", "crates/example/README.zh.md"]
        for rel in self.docs:
            self.write(rel, "Package / 包：`1.2.3`.\n")
        for rel in self.docs[:2]:
            self.write(rel, self.read(rel) + "`NDJSON_SCHEMA_VERSION = 2`\n`SDK_SCHEMA_VERSION = 3`\n")
        self.write("crates/opi-coding-agent/src/runner.rs", "pub const NDJSON_SCHEMA_VERSION: u32 = 2;\n")
        self.write("crates/opi-agent/src/sdk.rs", "pub const SDK_SCHEMA_VERSION: u32 = 3;\n")
        self.write("AGENTS.md", "Inspect the evidence; finish the agreed work.\n")
        self.write("CLAUDE.md", self.read("AGENTS.md"))

    def write(self, rel: str, content: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def read(self, rel: str) -> str:
        return (self.root / rel).read_text(encoding="utf-8")

    def test_version_values_do_not_require_fixed_sentences(self) -> None:
        doc_check.check_current_contracts(self.docs)
        self.assertEqual([], doc_check.ERRORS)

    def test_each_readme_requires_the_current_version(self) -> None:
        for rel in self.docs:
            with self.subTest(rel=rel):
                original = self.read(rel)
                self.write(rel, original.replace("1.2.3", "1.2.30"))
                doc_check.check_current_contracts(self.docs)
                self.assertTrue(any(error.startswith(f"{rel}:") and "version" in error for error in doc_check.ERRORS))
                doc_check.ERRORS.clear()
                self.write(rel, original)

    def test_schema_value_mismatch_is_rejected_even_with_matching_prefix(self) -> None:
        for rel in self.docs[:2]:
            for name, value in (("NDJSON_SCHEMA_VERSION", 2), ("SDK_SCHEMA_VERSION", 3)):
                with self.subTest(rel=rel, name=name):
                    original = self.read(rel)
                    self.write(rel, original.replace(f"{name} = {value}", f"{name} = {value}0"))
                    doc_check.check_current_contracts(self.docs)
                    self.assertTrue(any(error.startswith(f"{rel}:") and name in error for error in doc_check.ERRORS))
                    doc_check.ERRORS.clear()
                    self.write(rel, original)

    def test_schema_assignment_allows_whitespace_changes(self) -> None:
        for rel in self.docs[:2]:
            self.write(rel, self.read(rel).replace(" = ", "=\t"))
        doc_check.check_current_contracts(self.docs)
        self.assertEqual([], doc_check.ERRORS)

    def test_schema_assignments_accept_sentence_punctuation(self) -> None:
        for rel in self.docs[:2]:
            for name, value in (("NDJSON_SCHEMA_VERSION", 2), ("SDK_SCHEMA_VERSION", 3)):
                self.write(rel, self.read(rel).replace(f"`{name} = {value}`", f"{name} = {value}."))
        doc_check.check_current_contracts(self.docs)
        self.assertEqual([], doc_check.ERRORS)

    def test_schema_assignments_reject_numeric_and_identifier_suffixes(self) -> None:
        original = self.read("README.md")
        for invalid in ("2.5", "2_extra", "2suffix"):
            with self.subTest(invalid=invalid):
                self.write("README.md", original.replace("NDJSON_SCHEMA_VERSION = 2", f"NDJSON_SCHEMA_VERSION = {invalid}"))
                doc_check.check_current_contracts(self.docs)
                self.assertTrue(any("README.md:" in error and "NDJSON_SCHEMA_VERSION" in error for error in doc_check.ERRORS))
                doc_check.ERRORS.clear()

    def test_conflicting_schema_assignments_are_rejected(self) -> None:
        self.write("README.md", self.read("README.md") + "`NDJSON_SCHEMA_VERSION = 20`\n")
        doc_check.check_current_contracts(self.docs)
        self.assertTrue(any("README.md:" in error and "NDJSON_SCHEMA_VERSION" in error for error in doc_check.ERRORS))

    def test_missing_schema_assignment_is_rejected(self) -> None:
        self.write("README.md", "Package: `1.2.3`; `SDK_SCHEMA_VERSION = 3`.\n")
        doc_check.check_current_contracts(self.docs)
        self.assertTrue(any("README.md:" in error and "NDJSON_SCHEMA_VERSION" in error for error in doc_check.ERRORS))

    def test_missing_source_constant_is_rejected(self) -> None:
        self.write("crates/opi-agent/src/sdk.rs", "// No schema declaration.\n")
        doc_check.check_current_contracts(self.docs)
        self.assertTrue(any("cannot resolve SDK_SCHEMA_VERSION" in error for error in doc_check.ERRORS))

    def test_workspace_version_does_not_leak_from_another_toml_table(self) -> None:
        self.write("Cargo.toml", '[workspace.package]\nedition = "2024"\n[workspace.metadata]\nversion = "9.9.9"\n')
        self.assertEqual("", doc_check.workspace_version())
        self.assertTrue(any("workspace.package" in error for error in doc_check.ERRORS))

    def test_invalid_manifest_is_reported_without_a_traceback(self) -> None:
        self.write("crates/example/Cargo.toml", "[package\n")
        self.assertEqual([], doc_check.crate_packages())
        self.assertTrue(any("crates/example/Cargo.toml:" in error and "TOML" in error for error in doc_check.ERRORS))

    def test_counterparts_include_new_crates_and_reject_missing_translation(self) -> None:
        self.assertEqual(set(self.docs), set(doc_check.check_counterparts()))
        self.assertEqual([], doc_check.ERRORS)
        (self.root / "crates/example/README.zh.md").unlink()
        doc_check.check_counterparts()
        self.assertIn("missing file: crates/example/README.zh.md", doc_check.ERRORS)

    def test_empty_and_non_utf8_documents_are_rejected(self) -> None:
        self.write("README.md", " \n")
        (self.root / "README.zh.md").write_bytes(b"\xff")
        doc_check.check_counterparts()
        self.assertIn("empty document: README.md", doc_check.ERRORS)
        self.assertTrue(any(error.startswith("not UTF-8: README.zh.md:") for error in doc_check.ERRORS))

    def test_root_guidance_content_must_match(self) -> None:
        doc_check.check_root_guidance_lockstep()
        self.assertEqual([], doc_check.ERRORS)
        self.write("CLAUDE.md", "A different instruction.\n")
        doc_check.check_root_guidance_lockstep()
        self.assertTrue(any("AGENTS.md and CLAUDE.md diverge" in error for error in doc_check.ERRORS))

    def test_materialized_root_alias_warns_without_rejecting_checkout(self) -> None:
        self.write("CLAUDE.md", "AGENTS.md\n")
        warning = io.StringIO()
        with contextlib.redirect_stderr(warning):
            doc_check.check_root_guidance_lockstep()
        self.assertEqual([], doc_check.ERRORS)
        self.assertIn("materialized symlink", warning.getvalue())

    def test_main_accepts_free_prose_without_workflow_or_runtime_snippets(self) -> None:
        for rel in doc_check.CURRENT_CONTEXT_DOCS:
            self.write(rel, "# Current documentation\n")
        for rel in (".claude/skills/README.md", ".claude/skills/README.zh.md"):
            self.write(rel, "# Skills\n")
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            result = doc_check.main()
        self.assertEqual(0, result, "\n".join(doc_check.ERRORS))


class LocalLinkExemptionTest(unittest.TestCase):
    def test_only_repo_evidence_cache_links_are_exempt(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs").mkdir()
            (root / "docs/example.md").write_text(
                "[Cached evidence](../.repo/pi/evidence.md)\n"
                "[Current contract](missing.md)\n",
                encoding="utf-8",
            )
            with patch.object(doc_check, "ROOT", root), patch.object(doc_check, "ERRORS", []):
                doc_check.check_local_links("docs/example.md")
                self.assertEqual(
                    ["docs/example.md: broken local link: missing.md"],
                    doc_check.ERRORS,
                )


if __name__ == "__main__":
    unittest.main()
