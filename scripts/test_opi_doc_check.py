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


MINIMUM_CHANGE_TRACE_REQUIRED = {
    ".claude/skills/opi-implement/skill.md": (
        "**Minimum-change trace rule:**",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`legacy-unrecorded`",
        "`production_consumers`",
        "`nonproduction_consumers`",
        "`net_deletion`",
        "`residual_glue`",
        "simplification_trigger=",
    ),
    ".claude/skills/opi-implement/references/initializer.md": (
        "#### Minimum-change trace",
        '`field = "reuse_search"`',
        '`field = "placement"`',
        '`field = "surface_necessity"`',
        '`field = "simplification_ceiling"`',
        "`revisit_when`",
        "transitive `depends_on` closure",
        "REFUSE `confirm-all`",
        "`production_consumers=`",
        "`nonproduction_consumers=`",
        "`net_deletion=`",
        "`residual_glue=`",
        "simplification_trigger=",
    ),
    ".claude/skills/opi-implement/references/ledger-schema.md": (
        '`field = "reuse_search"`',
        "`searched=`",
        "`reused=`",
        "`gap=`",
        '`field = "placement"`',
        "`cannot_fit_fully=`",
        '`field = "surface_necessity"`',
        "`public_api=`",
        "`config=`",
        "`state=`",
        "`dependency_edge=`",
        '`field = "simplification_ceiling"`',
        "`ceiling=`",
        "`revisit_when=`",
        "`production_consumers=`",
        "`nonproduction_consumers=`",
        "`net_deletion=`",
        "`residual_glue=`",
        "simplification_trigger=",
    ),
    ".claude/skills/opi-implement/references/verify-engine.md": (
        "### Minimum-change trace overlay",
        "`reuse_search`",
        "`placement`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`revisit_when`",
        "`GRAPH_REVISION_REQUIRED`",
        "`RESEARCH_REQUIRED`",
        "`DESIGN_DECISION_REQUIRED`",
        "`production_consumers`",
        "`nonproduction_consumers`",
        "`net_deletion`",
        "`residual_glue`",
        "`simplification_trigger`",
    ),
    ".claude/skills/opi-implement/scripts/plan.workflow.js": (
        '"reuse_search"',
        '"placement"',
        '"surface_necessity"',
        '"simplification_ceiling"',
        "revisit_when",
        "transitive depends_on closure",
        "require production_consumers",
        "nonproduction_consumers",
        "net_deletion",
        "residual_glue",
        "simplification_trigger",
    ),
}


AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED = {
    ".claude/skills/opi-audit/SKILL.md": (
        "minimum-change conformance matrix",
        "`reuse_search`",
        "`surface_necessity`",
        "`simplification_ceiling`",
        "`conforming`",
        "`drifted`",
        "`triggered`",
        "`not-recorded`",
        "`not-assessable`",
        "current committed `audit_head`",
        "Finding routing remains on existing axes",
        "## N+2. Minimum-change Conformance",
        "`production_consumers`",
        "`nonproduction_consumers`",
        "`net_deletion`",
        "`residual_glue`",
    ),
    ".claude/skills/opi-audit/references/finding-template.md": (
        "## Minimum-change Conformance",
        "| Task | Scenario/source | Reuse | Placement | Surface | Production slice | Ceiling/trigger | Status |",
        "`not-recorded`",
        "`not-assessable`",
        "`standards`",
        "`spec`",
        "`integration`",
        "`production_consumers`",
        "`nonproduction_consumers`",
        "`net_deletion`",
        "`residual_glue`",
    ),
}


EVAL_BEHAVIOR_BASELINE_REQUIRED = {
    ".claude/skills/opi-eval/SKILL.md": (
        "sole deterministic acceptance baseline",
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`case_id@revision + provider:model + OS/arch + run_mode + effective_tools`",
        "`incomparable`",
        "`record-only`",
        "`evaluator_model`",
        "`independence`",
        "Do not create a behavior-baseline manifest",
    ),
    ".claude/skills/opi-eval/references/test-cases.md": (
        "`provider-fidelity`",
        "`runtime-fidelity`",
        "`candy@1`",
        "`tool_chain@1`",
        "`context_retention@1`",
        "`criterion/scenario reference`",
        "`fidelity justification`",
    ),
    ".claude/skills/opi-eval/references/evaluator-prompt.md": (
        "fidelity signal, not deterministic acceptance evidence",
        "same comparison identity",
        "`incomparable`",
        "`record-only`",
        "do not calculate a delta",
        "must not affect the overall verdict",
    ),
    ".claude/skills/opi-eval/references/report-template.md": (
        "**Case class**",
        "**Case revision**",
        "**Criterion/scenario**",
        "**Comparison identity**",
        "**Comparison status**",
        "`record-only`",
    ),
    "docs/eval/README.md": (
        "not deterministic acceptance evidence",
        "`case_id`",
        "`case_class`",
        "`case_revision`",
        "`criterion_source`",
        "`comparison_identity`",
        "`comparison_status`",
        "`evaluator_model`",
        "`independence`",
    ),
}


OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED = {
    "docs/opi-spec.md": (
        "resolved execution",
        "benchmark integrity",
        "model-visible content",
        "exact immutable package artifact digest",
        "ordinary-context/no-memory baseline",
        "Proactive or scheduled Agent behavior",
        "Multi-Agent orchestration",
    ),
    "docs/opi-spec.zh.md": (
        "已解析执行",
        "基准完整性",
        "模型可见内容",
        "精确的不可变 package artifact digest",
        "普通上下文/无记忆基线",
        "主动式或定时 Agent 行为",
        "多 Agent 编排",
    ),
}


OPI_DOCUMENT_PROSE_CONTRACT_REQUIRED = {
    ".claude/skills/opi-document/SKILL.md": (
        "references/prose-contract.md",
        "human-facing prose",
        "complete proposition",
        "semantic judgment",
        "targeted scope",
    ),
    ".claude/skills/opi-document/references/prose-contract.md": (
        "# Prose contract",
        "## Scope and exclusions",
        "## Preserve the complete proposition",
        "## Owner-first editing",
        "`keep`",
        "`add`",
        "`trim`",
        "`restore`",
        "`restructure`",
        "`defer`",
        "`docs/snapshots/`",
        "Mechanical checks do not prove semantic quality",
        "model-visible",
    ),
    ".claude/skills/opi-document/references/documentation-checks.md": (
        "Semantic prose quality",
        "`references/prose-contract.md`",
        "semantic judgment",
    ),
}


CHANGE_SCOPE_CHECK_SELECTION_REQUIRED = {
    ".claude/skills/_shared/references/change-scope-and-check-selection.md": (
        "# Change scope and check selection",
        "`verified_base=<explicit live base ref>`",
        "`head=<ref>`",
        "`git merge-base <verified-base> <head>`",
        "`git diff --name-status --find-renames <merge-base>..<head>`",
        "`git diff --cached --name-status`",
        "`git diff --name-status`",
        "`git ls-files --others --exclude-standard`",
        "record `committed`, `staged`, `unstaged`, and `untracked` separately",
        "`check-selection-only`",
        "does not bound audit coverage",
        "does not define task ownership",
        "does not define release manifest",
        "Do not rerun unchanged evidence",
        "worktree-only",
    ),
    "AGENTS.md": (
        ".agents/skills/_shared/references/change-scope-and-check-selection.md",
        "ordinary non-skill work",
        "`check-selection-only`",
    ),
    ".claude/skills/opi-document/SKILL.md": (
        "_shared/references/change-scope-and-check-selection.md",
        "candidate discovery only",
        "documentation authority",
    ),
    ".claude/skills/opi-remediate/SKILL.md": (
        "_shared/references/change-scope-and-check-selection.md",
        "normalized findings and derived layers",
        "verification union",
    ),
    ".claude/skills/opi-slim-tests/SKILL.md": (
        "_shared/references/change-scope-and-check-selection.md",
        "Cargo metadata and complete test bodies",
        "post-change focused gates",
    ),
}


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

    def write_minimum_change_trace_docs(self) -> None:
        for rel, tokens in MINIMUM_CHANGE_TRACE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

    def write_audit_minimum_change_conformance_docs(self) -> None:
        for rel, tokens in AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

    def write_eval_behavior_baseline_docs(self) -> None:
        for rel, tokens in EVAL_BEHAVIOR_BASELINE_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

    def write_opi_spec_evidence_refinement_docs(self) -> None:
        for rel, tokens in OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

    def write_opi_document_prose_contract_docs(self) -> None:
        for rel, tokens in OPI_DOCUMENT_PROSE_CONTRACT_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

    def write_change_scope_check_selection_docs(self) -> None:
        for rel, tokens in CHANGE_SCOPE_CHECK_SELECTION_REQUIRED.items():
            self.write(rel, "\n".join(tokens) + "\n")

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

    def test_minimum_change_trace_contract_passes(self) -> None:
        self.write_minimum_change_trace_docs()

        checker = getattr(doc_check, "check_minimum_change_trace_contract", None)
        self.assertIsNotNone(checker, "minimum-change trace checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_minimum_change_trace_contract_requires_every_token(self) -> None:
        checker = getattr(doc_check, "check_minimum_change_trace_contract", None)
        self.assertIsNotNone(checker, "minimum-change trace checker must exist")

        for rel, tokens in MINIMUM_CHANGE_TRACE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_minimum_change_trace_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: minimum-change trace contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )

    def test_audit_minimum_change_conformance_contract_passes(self) -> None:
        self.write_audit_minimum_change_conformance_docs()

        checker = getattr(
            doc_check,
            "check_audit_minimum_change_conformance_contract",
            None,
        )
        self.assertIsNotNone(checker, "audit conformance checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_audit_minimum_change_conformance_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_audit_minimum_change_conformance_contract",
            None,
        )
        self.assertIsNotNone(checker, "audit conformance checker must exist")

        for rel, tokens in AUDIT_MINIMUM_CHANGE_CONFORMANCE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_audit_minimum_change_conformance_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: audit minimum-change conformance contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )

    def test_eval_behavior_baseline_contract_passes(self) -> None:
        self.write_eval_behavior_baseline_docs()

        checker = getattr(
            doc_check,
            "check_eval_behavior_baseline_contract",
            None,
        )
        self.assertIsNotNone(checker, "eval behavior-baseline checker must exist")
        checker()

        self.assertEqual([], doc_check.ERRORS)

    def test_eval_behavior_baseline_contract_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_eval_behavior_baseline_contract",
            None,
        )
        self.assertIsNotNone(checker, "eval behavior-baseline checker must exist")

        for rel, tokens in EVAL_BEHAVIOR_BASELINE_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_eval_behavior_baseline_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token) + "\n",
                    )

                    checker()

                    self.assertIn(
                        f"{rel}: eval behavior-baseline contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )

    def test_opi_spec_evidence_refinement_contract_passes(self) -> None:
        self.write_opi_spec_evidence_refinement_docs()
        checker = getattr(
            doc_check,
            "check_opi_spec_evidence_refinement_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi spec evidence checker must exist")
        checker()
        self.assertEqual([], doc_check.ERRORS)

    def test_opi_spec_evidence_refinement_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_opi_spec_evidence_refinement_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi spec evidence checker must exist")
        for rel, tokens in OPI_SPEC_EVIDENCE_REFINEMENT_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_opi_spec_evidence_refinement_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token)
                        + "\n",
                    )
                    checker()
                    self.assertIn(
                        f"{rel}: Opi spec evidence refinement contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )

    def test_opi_document_prose_contract_passes(self) -> None:
        self.write_opi_document_prose_contract_docs()
        checker = getattr(
            doc_check,
            "check_opi_document_prose_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi document prose checker must exist")
        checker()
        self.assertEqual([], doc_check.ERRORS)

    def test_opi_document_prose_contract_requires_every_token(self) -> None:
        checker = getattr(
            doc_check,
            "check_opi_document_prose_contract",
            None,
        )
        self.assertIsNotNone(checker, "Opi document prose checker must exist")
        for rel, tokens in OPI_DOCUMENT_PROSE_CONTRACT_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_opi_document_prose_contract_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token)
                        + "\n",
                    )
                    checker()
                    self.assertIn(
                        f"{rel}: Opi document prose contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
                    )

    def test_change_scope_check_selection_contract_passes(self) -> None:
        self.write_change_scope_check_selection_docs()
        checker = getattr(
            doc_check,
            "check_change_scope_check_selection_contract",
            None,
        )
        self.assertIsNotNone(checker, "change-scope checker must exist")
        checker()
        self.assertEqual([], doc_check.ERRORS)

    def test_change_scope_check_selection_contract_requires_every_token(
        self,
    ) -> None:
        checker = getattr(
            doc_check,
            "check_change_scope_check_selection_contract",
            None,
        )
        self.assertIsNotNone(checker, "change-scope checker must exist")
        for rel, tokens in CHANGE_SCOPE_CHECK_SELECTION_REQUIRED.items():
            for token in tokens:
                with self.subTest(rel=rel, token=token):
                    doc_check.ERRORS = []
                    self.write_change_scope_check_selection_docs()
                    self.write(
                        rel,
                        "\n".join(item for item in tokens if item != token)
                        + "\n",
                    )
                    checker()
                    self.assertIn(
                        f"{rel}: change-scope and check-selection contract "
                        f"missing semantic tokens {[token]!r}",
                        doc_check.ERRORS,
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

    def test_lowercase_skill_entry_is_supported(self) -> None:
        self.write_skill(entry="skill.md")
        self.write_indexes(("opi-example",))

        paths = doc_check.check_skill_contracts()

        self.assertEqual([], doc_check.ERRORS)
        self.assertIn(".claude/skills/opi-example/skill.md", paths)

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


class LocalLinkExemptionTest(unittest.TestCase):
    def test_repo_evidence_cache_links_are_exempt(self):
        """Links into the gitignored `.repo/` evidence cache must not be
        reported broken: the cache is non-normative local material and is
        absent from a fresh checkout by design."""
        doc_check.ERRORS.clear()
        for spec in ("docs/opi-spec.md", "docs/opi-spec.zh.md"):
            doc_check.check_local_links(spec)
        repo_link_errors = [e for e in doc_check.ERRORS if ".repo" in e]
        self.assertEqual(
            [],
            repo_link_errors,
            "links into the .repo evidence cache must be exempt from "
            "existence checks",
        )


if __name__ == "__main__":
    unittest.main()
