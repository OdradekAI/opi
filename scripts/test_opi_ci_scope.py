#!/usr/bin/env python3
"""Offline CI routing checks; no Cargo, credentials, or hosted runners."""

from __future__ import annotations

import importlib.util
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


SCRIPT = Path(__file__).with_name("opi-ci-scope.py")
SPEC = importlib.util.spec_from_file_location("opi_ci_scope", SCRIPT)
assert SPEC and SPEC.loader
ci = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ci)
SHA = "a" * 40


class ScopeTests(unittest.TestCase):
    def test_docs_and_eval_do_not_build_product(self):
        for paths in ([], ["README.md", ".agents/skills/opi-release/SKILL.md"],
                      ["crates/opi-eval/Cargo.toml", "crates/opi-eval/src/main.rs"],
                      [".agents/skills/opi-eval/scripts/operator.py"]):
            with self.subTest(paths=paths):
                self.assertEqual(ci.classify(paths), {"rust": False, "windows": False, "macos": False})

    def test_ordinary_code_defaults_to_linux(self):
        self.assertEqual(ci.classify(["crates/opi-ai/src/messages.rs"]),
                         {"rust": True, "windows": False, "macos": False})

    def test_platform_sensitive_paths_select_native_smoke(self):
        for path in ("Cargo.lock", "Cargo.toml", ".cargo/config.toml", "crates/opi-ai/Cargo.toml",
                     "crates/opi-sandbox/src/lib.rs", "crates/opi-tui/src/input.rs",
                     "crates/opi-ai/tests/auth_process_lifecycle.rs", ".github/workflows/ci.yml"):
            with self.subTest(path=path):
                self.assertEqual(ci.classify([path]), {"rust": True, "windows": True, "macos": True})

    def test_platform_specific_paths_do_not_select_unrelated_host(self):
        self.assertEqual(ci.classify(["crates/opi-ai/src/windows_store.rs"]),
                         {"rust": True, "windows": True, "macos": False})
        self.assertEqual(ci.classify(["crates/opi-ai/src/macos_store.rs"]),
                         {"rust": True, "windows": False, "macos": True})

    def test_powershell_changes_select_windows(self):
        self.assertEqual(ci.classify([".agents/skills/opi-implement/scripts/opi-impl-smoke.ps1"]),
                         {"rust": True, "windows": True, "macos": False})

    def test_unknown_scripts_fail_toward_product_checks(self):
        self.assertTrue(ci.classify(["scripts/new-check.py"])["rust"])

    def test_mixed_change_is_not_hidden_by_eval_or_documentation(self):
        self.assertTrue(ci.classify(["docs/ci.md", "crates/opi-eval/src/lib.rs", "crates/opi-agent/src/lib.rs"])["rust"])

    def test_matrix_is_exactly_the_requested_subset(self):
        for bits in range(1, 8):
            enabled = [bool(bits & (1 << bit)) for bit in range(3)]
            expected = [runner for runner, selected in zip(ci.RUNNERS.values(), enabled) if selected]
            self.assertEqual(ci.selected_matrix(*enabled), {"os": expected})
        with self.assertRaisesRegex(ValueError, "at least one"):
            ci.selected_matrix(False, False, False)

    def test_deleted_and_new_paths_both_participate(self):
        result = subprocess.CompletedProcess([], 0, b"crates/opi-ai/src/windows.rs\0docs/moved.md\0")
        with patch.object(ci.subprocess, "run", return_value=result) as run:
            paths = ci.git_paths("b" * 40, SHA)
        self.assertTrue(ci.classify(paths)["windows"])
        self.assertIn("--no-renames", run.call_args_list[0].args[0])
        self.assertEqual(run.call_args_list[1].args[0][:3], ["git", "diff", "--check"])

    def test_initial_push_uses_the_committed_tree(self):
        with patch.object(ci.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, b"Cargo.toml\0")) as run:
            self.assertEqual(ci.git_paths("0" * 40, SHA), ["Cargo.toml"])
        self.assertEqual(run.call_args_list[0].args[0][:2], ["git", "ls-tree"])

    def test_missing_git_history_is_not_a_docs_only_success(self):
        with patch.object(ci.subprocess, "run", side_effect=subprocess.CalledProcessError(128, "git")):
            with self.assertRaises(subprocess.CalledProcessError):
                ci.git_paths("b" * 40, SHA)

    def test_invalid_git_ref_is_rejected_before_git(self):
        with patch.object(ci.subprocess, "run") as run:
            for value in ("main", "HEAD", "--help", "a" * 39):
                with self.assertRaises(ValueError):
                    ci.git_paths(value, SHA)
            run.assert_not_called()

    def test_matrix_cli_writes_valid_action_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "outputs"
            result = subprocess.run([sys.executable, str(SCRIPT), "matrix", "--ref", SHA,
                                     "--windows", "true", "--output", str(output)],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            key, value = output.read_text().strip().split("=", 1)
            self.assertEqual(key, "matrix")
            self.assertEqual(json.loads(value), {"os": ["windows-latest"]})

    def test_scope_cli_reads_real_delta_and_rejects_whitespace_errors(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
            for key in ("GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"):
                env.pop(key, None)

            def git(*arguments):
                result = subprocess.run(["git", "-c", "user.name=CI fixture", "-c",
                                         "user.email=fixture@example.invalid", "-c", "core.autocrlf=false",
                                         "-c", f"core.hooksPath={root / 'no-hooks'}", *arguments], cwd=root, env=env, check=True, capture_output=True, text=True)
                return result.stdout.strip()

            def commit():
                git("add", "--", "README.md")
                if (root / "windows.rs").exists():
                    git("add", "--", "windows.rs")
                git("commit", "-m", "fixture")
                return git("rev-parse", "HEAD")

            def scope(base, head):
                return subprocess.run([sys.executable, str(SCRIPT), "scope", "--base", base, "--head", head],
                                      cwd=root, env=env, capture_output=True, text=True, timeout=10)

            git("init")
            (root / "README.md").write_text("fixture\n")
            initial = commit()
            (root / "README.md").write_text("updated\n")
            docs = commit()
            result = scope(initial, docs)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), dict(rust=False, windows=False, macos=False))
            (root / "windows.rs").write_text("// fixture\n")
            product = commit()
            for base in (docs, "0" * 40):
                result = scope(base, product)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(json.loads(result.stdout), dict(rust=True, windows=True, macos=False))
            (root / "README.md").write_text("invalid  \n")
            invalid = commit()
            result = scope(product, invalid)
            self.assertEqual(result.returncode, 2)
            self.assertIn("CI selection failed", result.stderr)

    def test_invalid_matrix_cli_does_not_emit_success(self):
        for arguments in (["--ref", SHA], ["--ref", "main", "--linux", "true"],
                          ["--ref", SHA, "--linux", "yes"]):
            result = subprocess.run([sys.executable, str(SCRIPT), "matrix", *arguments],
                                    capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")


def workflow_step(filename: str, name: str) -> str:
    source = (SCRIPT.parent.parent / ".github/workflows" / filename).read_text()
    block = source.split(f"- name: {name}\n", 1)[1].split("        run: |\n", 1)[1]
    lines = []
    for line in block.splitlines():
        if line and not line.startswith("          "):
            break
        lines.append(line[10:])
    return "\n".join(lines)


@unittest.skipIf(os.name == "nt" or not shutil.which("bash"), "POSIX shell execution")
class WorkflowExecutionTests(unittest.TestCase):
    def test_required_gate_rejects_failed_cancelled_or_missing_selected_jobs(self):
        script = workflow_step("ci.yml", "Require every selected check")
        baseline = dict(SCOPE="success", RUST="true", PRODUCT="success", MSRV="success",
                        WINDOWS="true", MACOS="false", PLATFORMS="success")
        for key in ("SCOPE", "PRODUCT", "MSRV", "PLATFORMS"):
            for value in ("failure", "cancelled", "skipped", ""):
                env = dict(os.environ, **baseline)
                env[key] = value
                result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, timeout=10)
                self.assertNotEqual(result.returncode, 0, (key, value))
        for key in ("RUST", "WINDOWS", "MACOS"):
            for value in ("", "unknown"):
                env = dict(os.environ, **baseline)
                env[key] = value
                result = subprocess.run(["bash", "-c", script], env=env, capture_output=True, timeout=10)
                self.assertNotEqual(result.returncode, 0, (key, value))
        for overrides in ({}, dict(RUST="false", PRODUCT="skipped", MSRV="skipped",
                                    WINDOWS="false", PLATFORMS="skipped")):
            result = subprocess.run(["bash", "-c", script], env=dict(os.environ, **(baseline | overrides)),
                                    capture_output=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)

    def test_uploader_requires_draft_and_does_not_mask_upload_failure(self):
        script = workflow_step("release.yml", "Upload to GitHub Release")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "artifacts").mkdir()
            (root / "artifacts/opi-linux.tar.gz").write_text("fixture")
            (root / "artifacts/SHA256SUMS.txt").write_text("fixture")
            tools = root / "bin"
            tools.mkdir()
            gh = tools / "gh"
            gh.write_text('#!/bin/sh\nprintf "%s\\n" "$*" >> "$CALLS"\n'
                          'if [ "$2" = view ]; then\n'
                          '  [ "$DRAFT" != missing ] || exit 1\n'
                          '  echo "$DRAFT"; exit 0\nfi\nexit "$UPLOAD_EXIT"\n')
            gh.chmod(0o755)
            sleep = tools / "sleep"
            sleep.write_text("#!/bin/sh\nexit 0\n")
            sleep.chmod(0o755)
            log = root / "calls"
            for draft, upload_exit, success in (("true", "0", True), ("false", "0", False),
                                                ("missing", "0", False), ("true", "42", False)):
                log.write_text("")
                env = dict(os.environ, PATH=str(tools) + os.pathsep + "/usr/bin:/bin",
                           RELEASE_TAG="v0.0.0", DRAFT=draft, UPLOAD_EXIT=upload_exit, CALLS=str(log))
                result = subprocess.run(["bash", "-c", script], cwd=root, env=env,
                                        capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
                calls = log.read_text().splitlines()
                uploads = [line for line in calls if line.startswith("release upload ")]
                self.assertEqual(bool(uploads), draft == "true")
                self.assertNotIn("--clobber", "\n".join(calls))
                if draft == "missing":
                    self.assertEqual(len(calls), 12)
                if upload_exit == "42":
                    self.assertEqual(result.returncode, 42)

    def test_scope_cli_missing_history_refuses_without_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            result = subprocess.run([sys.executable, str(SCRIPT), "scope", "--base", "b" * 40,
                                     "--head", SHA], cwd=temporary, capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main(verbosity=2)
