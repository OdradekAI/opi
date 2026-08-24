"""Coordinator tests: independent member installation into the live audit set."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = ROOT / ".agents" / "skills" / "_shared" / "scripts"
SCRIPT = SCRIPTS / "assurance_set.py"
VALIDATOR = SCRIPTS / "validate_assurance_artifact.py"
GENERATION_ID = "phase17-legacy-20260824t010203z"


def run_script(script: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(script), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def run_generation(*args: str) -> subprocess.CompletedProcess[str]:
    return run_script(SCRIPT, *args)


def run_validator(kind: str, path: Path) -> subprocess.CompletedProcess[str]:
    return run_script(VALIDATOR, kind, str(path))


def run_git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(repo), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def write_jsonl(path: Path, records: list[object]) -> None:
    path.write_text(
        "".join(json.dumps(record, sort_keys=True) + "\n" for record in records),
        encoding="utf-8",
        newline="\n",
    )


def requirement_record(run_id: str) -> dict[str, object]:
    return {
        "audit_run_id": run_id,
        "id": "P17-A1",
        "mandatory": True,
        "criterion_source": {
            "path": "docs/opi-spec.md",
            "sha256": "a" * 64,
            "citation": "P17-A1",
        },
        "observable_behavior": "The registered behavior is present.",
        "production_surfaces": ["crates/opi-agent/src/lib.rs"],
        "test_evidence": ["phase17_api_audit"],
        "checks": [
            {"command": "cargo test -p opi-agent phase17_api_audit", "observed": "PASS"}
        ],
        "state": "met",
        "finding_ids": [],
    }


def member_run_id(reviewer: str, model: str, stamp: str = "1") -> str:
    return f"phase17-{reviewer}-{model}-run{stamp}"


class SetFixture(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        for args in (
            ("init",),
            ("config", "user.email", "test@example.com"),
            ("config", "user.name", "Assurance Test"),
        ):
            result = run_git(self.root, *args)
            self.assertEqual(0, result.returncode, result.stderr)
        self.phase_dir = self.root / "docs" / "snapshots" / "phase17"
        self.assurance_dir = self.phase_dir / "assurance"
        self.phase_dir.mkdir(parents=True)
        (self.phase_dir / "opi-impl-state.json").write_text("{}\n", encoding="utf-8")
        run_git(self.root, "add", "docs/snapshots/phase17/opi-impl-state.json")
        run_git(self.root, "commit", "-m", "phase ledger", "--quiet")
        self.head_base = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        (self.root / "workspace.txt").write_text("change\n", encoding="utf-8")
        run_git(self.root, "add", "workspace.txt")
        run_git(self.root, "commit", "-m", "workspace", "--quiet")
        self.head_head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        self.staging_root = self.root / "member-staging"
        self.staging_root.mkdir()

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write_member(
        self,
        reviewer: str,
        model: str,
        *,
        run_id: str | None = None,
        audit_head: str | None = None,
        verdict: str = "PASS",
        schema_version: int = 3,
        generation_id: str | None = None,
        crlf: bool = False,
        extra_file: str | None = None,
        directory: Path | None = None,
    ) -> tuple[Path, dict[str, object]]:
        run = run_id or member_run_id(reviewer, model)
        head = audit_head or self.head_head
        stem = f"audit.{reviewer}.{model}"
        target = directory or (self.staging_root / f"{reviewer}-{model}")
        target.mkdir(parents=True, exist_ok=True)
        requirements_path = target / f"{stem}.requirements.jsonl"
        findings_path = target / f"{stem}.findings.jsonl"
        report_path = target / f"{stem}.md"
        meta_path = target / f"{stem}.meta.json"
        newline = "\r\n" if crlf else "\n"
        write_jsonl(requirements_path, [requirement_record(run)])
        write_jsonl(findings_path, [])
        meta: dict[str, object] = {
            "schema_version": schema_version,
            "audit_run_id": run,
            "phase": 17,
            "audit_head": head,
            "reviewer_id": reviewer,
            "reviewer_identity": reviewer.capitalize(),
            "model_id": model,
            "reviewer_model_id": f"{model}-model",
            "model_identity_source": "operator-declared",
            "independence": "fresh-context-same-family",
            "baseline_policy": "latest-committed-spec",
            "baseline_sources": [
                {"path": ".opi-impl-state.json", "sha256": "b" * 64},
                {"path": "docs/snapshots/phase17/opi-impl-state.json", "sha256": "c" * 64},
                {"path": "docs/opi-spec.md", "sha256": "d" * 64},
            ],
            "requirements_sha256": sha256(requirements_path),
            "findings_sha256": sha256(findings_path),
            "verdict": verdict,
        }
        if generation_id:
            meta["audit_generation_id"] = generation_id
        write_json(meta_path, meta)
        report = (
            "# Phase 17 Audit\n\n"
            f"**Audit run ID**: `{run}`\n"
            f"**Audit head**: `{head}`\n"
            f"**Reviewer ID**: `{reviewer}`\n"
            f"**Model ID**: `{model}`\n"
            f"**Reviewer identity**: `{reviewer.capitalize()}`\n"
            f"**Reviewer model ID**: `{model}-model`\n"
            "**Model identity source**: `operator-declared`\n"
            f"**Verdict**: {verdict}\n"
        )
        report_path.write_text(report, encoding="utf-8", newline=newline)
        if crlf:
            for path in (requirements_path, findings_path, meta_path):
                data = path.read_bytes().replace(b"\n", b"\r\n")
                path.write_bytes(data)
        if extra_file:
            (target / extra_file).write_text("stray\n", encoding="utf-8")
        entry = {
            "reviewer_id": reviewer,
            "model_id": model,
            "artifact_stem": stem,
            "audit_run_id": run,
            "audit_head": head,
            "verdict": verdict,
            "digests": {
                "meta_sha256": sha256(meta_path),
                "requirements_sha256": sha256(requirements_path),
                "findings_sha256": sha256(findings_path),
                "report_sha256": sha256(report_path),
            },
        }
        return target, entry

    def complete(self, directory: Path, reviewer: str, model: str):
        return run_generation(
            "complete",
            str(self.phase_dir),
            str(directory),
            "--reviewer",
            reviewer,
            "--model",
            model,
        )

    def commit_assurance(self, message: str) -> None:
        run_git(self.root, "add", "docs/snapshots/phase17/assurance")
        run_git(self.root, "commit", "-m", message, "--quiet")

    def install(self, reviewer: str, model: str, **kwargs) -> dict[str, object]:
        directory, entry = self.write_member(reviewer, model, **kwargs)
        result = self.complete(directory, reviewer, model)
        self.assertEqual(0, result.returncode, result.stderr)
        return entry

    def live_index(self) -> dict[str, object]:
        return json.loads(
            (self.assurance_dir / "audit.index.json").read_text(encoding="utf-8")
        )

    def build_v1_set(self) -> None:
        """Write and commit a legacy schema-1 generation set for migrate tests."""
        self.assurance_dir.mkdir(parents=True, exist_ok=True)
        directory, entry = self.write_member(
            "codex",
            "gpt56",
            generation_id=GENERATION_ID,
            schema_version=2,
        )
        for name in (
            f"audit.codex.gpt56.meta.json",
            "audit.codex.gpt56.requirements.jsonl",
            "audit.codex.gpt56.findings.jsonl",
            "audit.codex.gpt56.md",
        ):
            (self.assurance_dir / name).write_bytes((directory / name).read_bytes())
        legacy_entry = {key: value for key, value in entry.items()}
        write_json(
            self.assurance_dir / "audit.index.json",
            {
                "schema_version": 1,
                "phase": 17,
                "audit_generation_id": GENERATION_ID,
                "audit_head": entry["audit_head"],
                "revision": 1,
                "aggregate_verdict": "PASS",
                "members": [legacy_entry],
            },
        )
        self.commit_assurance("legacy set")


class CompleteTests(SetFixture):
    def test_complete_first_member_installs_live_files_and_index_v2(self) -> None:
        entry = self.install("codex", "gpt56", audit_head=self.head_base)

        index = self.live_index()
        self.assertEqual(2, index["schema_version"])
        self.assertEqual(1, index["revision"])
        self.assertEqual([entry], index["members"])
        for name in (
            "audit.codex.gpt56.meta.json",
            "audit.codex.gpt56.requirements.jsonl",
            "audit.codex.gpt56.findings.jsonl",
            "audit.codex.gpt56.md",
        ):
            self.assertTrue((self.assurance_dir / name).is_file(), name)
        result = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, result.returncode, result.stderr)

    def test_complete_second_member_appends_and_increments_revision(self) -> None:
        self.install("codex", "gpt56", audit_head=self.head_base)
        self.commit_assurance("first member")
        self.install("claude", "glm53", audit_head=self.head_head)

        index = self.live_index()
        self.assertEqual(2, index["revision"])
        self.assertEqual(2, len(index["members"]))
        self.assertEqual("PASS", index["aggregate_verdict"])
        result = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, result.returncode, result.stderr)

    def test_complete_replaces_own_slot_and_moves_old_files_to_history_run_id(
        self,
    ) -> None:
        self.install("codex", "gpt56", audit_head=self.head_base)
        self.commit_assurance("first member")
        old_entry = self.install(
            "claude", "glm53", run_id=member_run_id("claude", "glm53", "old")
        )
        self.commit_assurance("second member")
        new_entry = self.install(
            "claude", "glm53", run_id=member_run_id("claude", "glm53", "new")
        )

        index = self.live_index()
        self.assertEqual(3, index["revision"])
        self.assertEqual(
            ["claude", "codex"],
            [member["reviewer_id"] for member in index["members"]],
        )
        self.assertEqual(new_entry["audit_run_id"], index["members"][0]["audit_run_id"])
        history = self.assurance_dir / "history" / str(old_entry["audit_run_id"])
        for name in (
            "audit.claude.glm53.meta.json",
            "audit.claude.glm53.requirements.jsonl",
            "audit.claude.glm53.findings.jsonl",
            "audit.claude.glm53.md",
        ):
            self.assertTrue((history / name).is_file(), name)
        result = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, result.returncode, result.stderr)

    def test_complete_replacement_leaves_other_members_untouched(self) -> None:
        self.install("codex", "gpt56", audit_head=self.head_base)
        self.commit_assurance("first member")
        codex_before = (
            self.assurance_dir / "audit.codex.gpt56.md"
        ).read_bytes()
        self.install("claude", "glm53", run_id=member_run_id("claude", "glm53", "a"))
        self.commit_assurance("second member")
        self.install("claude", "glm53", run_id=member_run_id("claude", "glm53", "b"))

        self.assertEqual(
            codex_before, (self.assurance_dir / "audit.codex.gpt56.md").read_bytes()
        )
        self.assertTrue((self.assurance_dir / "audit.codex.gpt56.meta.json").is_file())

    def test_complete_refuses_non_ancestor_audit_head(self) -> None:
        run_git(self.root, "checkout", "-q", "-b", "side")
        (self.root / "side.txt").write_text("side\n", encoding="utf-8")
        run_git(self.root, "add", "side.txt")
        run_git(self.root, "commit", "-m", "side", "--quiet")
        side_head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        run_git(self.root, "checkout", "-q", "master")

        directory, _ = self.write_member("codex", "gpt56", audit_head=side_head)
        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("not a committed ancestor of HEAD", result.stderr)
        self.assertFalse((self.assurance_dir / "audit.index.json").exists())

    def test_complete_refuses_dirty_live_assurance_directory(self) -> None:
        self.install("codex", "gpt56", audit_head=self.head_base)
        self.commit_assurance("first member")
        report = self.assurance_dir / "audit.codex.gpt56.md"
        report.write_text(report.read_text(encoding="utf-8") + "dirty\n", encoding="utf-8")

        directory, _ = self.write_member("claude", "glm53")
        result = self.complete(directory, "claude", "glm53")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("admission refused", result.stderr)
        self.assertFalse((self.assurance_dir / "audit.claude.glm53.md").is_file())

    def test_complete_refuses_invalid_staged_member(self) -> None:
        directory, entry = self.write_member("codex", "gpt56")
        meta_path = directory / "audit.codex.gpt56.meta.json"
        meta = json.loads(meta_path.read_text(encoding="utf-8"))
        meta["phase"] = 16
        write_json(meta_path, meta)

        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("member validation failed", result.stderr)
        self.assertFalse((self.assurance_dir / "audit.index.json").exists())

    def test_complete_refuses_staging_inside_live_assurance(self) -> None:
        directory, _ = self.write_member(
            "codex", "gpt56", directory=self.assurance_dir / "inside"
        )
        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("outside the live assurance directory", result.stderr)

    def test_complete_requires_valid_live_set_before_append(self) -> None:
        self.install("codex", "gpt56", audit_head=self.head_base)
        self.commit_assurance("first member")
        findings = self.assurance_dir / "audit.codex.gpt56.findings.jsonl"
        findings.write_bytes(findings.read_bytes() + b"corrupted\n")
        self.commit_assurance("corrupt the live set digests")

        directory, _ = self.write_member("claude", "glm53")
        result = self.complete(directory, "claude", "glm53")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("current live audit set is invalid", result.stderr)

    def test_complete_refuses_legacy_index_v1_and_names_migrate(self) -> None:
        self.build_v1_set()
        directory, _ = self.write_member("claude", "glm53")

        result = self.complete(directory, "claude", "glm53")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("requires migrate", result.stderr)

    def test_complete_refuses_replay_of_installed_run_id(self) -> None:
        run = member_run_id("codex", "gpt56", "same")
        self.install("codex", "gpt56", run_id=run, audit_head=self.head_base)
        self.commit_assurance("first member")
        directory, _ = self.write_member("codex", "gpt56", run_id=run)

        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("audit run is already installed", result.stderr)

    def test_complete_refuses_crlf_staged_files(self) -> None:
        directory, _ = self.write_member("codex", "gpt56", crlf=True)

        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn("must use LF line endings only", result.stderr)

    def test_complete_refuses_extra_files_in_member_directory(self) -> None:
        directory, _ = self.write_member("codex", "gpt56", extra_file="notes.txt")

        result = self.complete(directory, "codex", "gpt56")

        self.assertNotEqual(0, result.returncode)
        self.assertIn(
            "exactly the four audit.<reviewer>.<model>.* files", result.stderr
        )

    def test_concurrent_same_member_completion_has_one_winner(self) -> None:
        first, _ = self.write_member(
            "codex",
            "gpt56",
            run_id=member_run_id("codex", "gpt56", "one"),
        )
        second, _ = self.write_member(
            "codex",
            "gpt56",
            run_id=member_run_id("codex", "gpt56", "two"),
        )
        processes = [
            subprocess.Popen(
                [
                    sys.executable,
                    str(SCRIPT),
                    "complete",
                    str(self.phase_dir),
                    str(directory),
                    "--reviewer",
                    "codex",
                    "--model",
                    "gpt56",
                ],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            for directory in (first, second)
        ]
        codes = [process.wait() for process in processes]
        for process in processes:
            process.stdout.close()
            process.stderr.close()

        self.assertEqual(1, sum(1 for code in codes if code == 0), codes)
        index = self.live_index()
        self.assertEqual(1, len(index["members"]))


class RecoveryTests(SetFixture):
    def load_module(self):
        sys.path.insert(0, str(SCRIPTS))
        try:
            import assurance_set as module
        finally:
            sys.path.remove(str(SCRIPTS))
        return module

    def interrupted_install(self, module, *, install_index: bool, prior_member: bool):
        """Prepare and partially execute a member install, then abandon it."""
        directory, entry = self.write_member("codex", "gpt56")
        if prior_member:
            self.install("claude", "glm53")
            self.commit_assurance("first member")
        with module.AssuranceLock(self.root, 17):
            transaction, journal = module.prepare_member_install(
                self.root, self.phase_dir, directory, entry
            )
            journal["state"] = "installing"
            module.atomic_write_json(transaction / "journal.json", journal)
            for name in journal["member_files"]:
                module.atomic_copy(
                    transaction / "new" / name, self.assurance_dir / name
                )
            if install_index:
                module.atomic_copy(
                    transaction / "new" / "audit.index.json",
                    self.assurance_dir / "audit.index.json",
                )
        return transaction

    def test_recover_restores_prior_state_after_install_interruption(self) -> None:
        module = self.load_module()
        self.interrupted_install(module, install_index=False, prior_member=True)

        result = run_generation("recover", str(self.phase_dir))
        recovered = sorted(
            path.name for path in self.assurance_dir.iterdir() if path.is_file()
        )

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertEqual(
            [
                "audit.claude.glm53.findings.jsonl",
                "audit.claude.glm53.md",
                "audit.claude.glm53.meta.json",
                "audit.claude.glm53.requirements.jsonl",
                "audit.index.json",
            ],
            recovered,
        )
        validation = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, validation.returncode, validation.stderr)

    def test_recover_restores_after_first_member_interruption(self) -> None:
        module = self.load_module()
        self.interrupted_install(module, install_index=True, prior_member=False)

        result = run_generation("recover", str(self.phase_dir))

        self.assertEqual(0, result.returncode, result.stderr)
        self.assertFalse((self.assurance_dir / "audit.index.json").exists())
        self.assertFalse(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").exists()
        )

    def test_live_validator_recovers_interrupted_install_before_read(self) -> None:
        module = self.load_module()
        self.interrupted_install(module, install_index=False, prior_member=True)

        result = run_validator("audit-set", self.assurance_dir)

        self.assertEqual(0, result.returncode, result.stderr)

    def test_recover_finalizes_history_after_switched_interruption(self) -> None:
        module = self.load_module()
        old_entry = self.install(
            "claude", "glm53", run_id=member_run_id("claude", "glm53", "old")
        )
        self.commit_assurance("first member")
        directory, entry = self.write_member(
            "claude", "glm53", run_id=member_run_id("claude", "glm53", "new")
        )
        with module.AssuranceLock(self.root, 17):
            transaction, journal = module.prepare_member_install(
                self.root, self.phase_dir, directory, entry
            )
            module.install_member(self.assurance_dir, transaction, journal)
            journal["state"] = "switched"
            module.atomic_write_json(transaction / "journal.json", journal)

        result = run_generation("recover", str(self.phase_dir))

        self.assertEqual(0, result.returncode, result.stderr)
        history = (
            self.assurance_dir / "history" / str(old_entry["audit_run_id"])
        )
        self.assertTrue((history / "audit.claude.glm53.md").is_file())
        validation = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, validation.returncode, validation.stderr)


class MigrateTests(SetFixture):
    def test_migrate_converts_v1_generation_to_live_set_v2(self) -> None:
        self.build_v1_set()
        requirements_before = (
            self.assurance_dir / "audit.codex.gpt56.requirements.jsonl"
        ).read_bytes()
        findings_before = (
            self.assurance_dir / "audit.codex.gpt56.findings.jsonl"
        ).read_bytes()

        result = run_generation("migrate", str(self.phase_dir))

        self.assertEqual(0, result.returncode, result.stderr)
        index = self.live_index()
        self.assertEqual(2, index["schema_version"])
        self.assertEqual(2, index["revision"])
        self.assertNotIn("audit_generation_id", index)
        member = index["members"][0]
        self.assertEqual(self.head_head, member["audit_head"])
        meta = json.loads(
            (self.assurance_dir / "audit.codex.gpt56.meta.json").read_text(
                encoding="utf-8"
            )
        )
        self.assertEqual(3, meta["schema_version"])
        self.assertNotIn("audit_generation_id", meta)
        report = (self.assurance_dir / "audit.codex.gpt56.md").read_text(
            encoding="utf-8"
        )
        self.assertNotIn("Audit generation ID", report)
        self.assertEqual(
            requirements_before,
            (self.assurance_dir / "audit.codex.gpt56.requirements.jsonl").read_bytes(),
        )
        self.assertEqual(
            findings_before,
            (self.assurance_dir / "audit.codex.gpt56.findings.jsonl").read_bytes(),
        )
        validation = run_validator("audit-set", self.assurance_dir)
        self.assertEqual(0, validation.returncode, validation.stderr)

    def test_migrate_refuses_missing_or_v2_index(self) -> None:
        result = run_generation("migrate", str(self.phase_dir))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("legacy index v1", result.stderr)

        self.build_v1_set()
        run_generation("migrate", str(self.phase_dir))
        self.commit_assurance("migrated")
        result = run_generation("migrate", str(self.phase_dir))
        self.assertNotEqual(0, result.returncode)
        self.assertIn("legacy index v1", result.stderr)

    def test_migrate_refuses_dirty_assurance_directory(self) -> None:
        self.build_v1_set()
        report = self.assurance_dir / "audit.codex.gpt56.md"
        report.write_text(report.read_text(encoding="utf-8") + "dirty\n", encoding="utf-8")

        result = run_generation("migrate", str(self.phase_dir))

        self.assertNotEqual(0, result.returncode)
        self.assertIn("admission refused", result.stderr)

    def test_migrate_is_recoverable_on_interruption(self) -> None:
        self.build_v1_set()
        before = {
            path.name: path.read_bytes()
            for path in self.assurance_dir.iterdir()
            if path.is_file()
        }
        sys.path.insert(0, str(SCRIPTS))
        try:
            import assurance_set as module
        finally:
            sys.path.remove(str(SCRIPTS))
        with module.AssuranceLock(self.root, 17):
            transaction, journal = module.prepare_set_rewrite(
                self.root, self.phase_dir
            )
            journal["state"] = "installing"
            module.atomic_write_json(transaction / "journal.json", journal)
            (self.assurance_dir / "audit.codex.gpt56.md").unlink()

        result = run_generation("recover", str(self.phase_dir))

        self.assertEqual(0, result.returncode, result.stderr)
        after = {
            path.name: path.read_bytes()
            for path in self.assurance_dir.iterdir()
            if path.is_file()
        }
        self.assertEqual(before, after)
        validation = run_validator("rotation", self.phase_dir)
        self.assertEqual(0, validation.returncode, validation.stderr)


if __name__ == "__main__":
    unittest.main()
