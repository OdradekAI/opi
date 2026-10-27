from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
from argparse import Namespace
from pathlib import Path
from unittest.mock import patch

import remediation_source_proof as proof_module
from remediation_source_proof import ProofError, repo_path

from test_opi_assurance_skills import (
    AssuranceFixture,
    REMEDIATION_HEAD,
    finding_record,
    member_run_id,
    requirement_record,
    run_git,
    sha256,
    write_jsonl,
)


HELPER = Path(__file__).with_name("remediation_source_proof.py")


class SourceProofFixture(AssuranceFixture):
    commit_live_audit = True

    def setUp(self) -> None:
        super().setUp()
        self.external = tempfile.TemporaryDirectory(prefix="opi proof receipts ")
        self.addCleanup(self.external.cleanup)
        self.receipts = Path(self.external.name)
        self.source = self.root / "crates/opi-agent/src/session.rs"
        self.source.parent.mkdir(parents=True)
        self.source.write_text("original source\n", encoding="utf-8")
        self.research = self.root / "research notes.txt"
        self.research.write_text("original research\n", encoding="utf-8")
        self.commit_paths(self.source, self.research)
        audit_head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        run_id = member_run_id("codex", "gpt56")
        member = self.write_audit_member(
            "codex", "gpt56", "gpt-5.6", audit_head=audit_head,
            requirements=[requirement_record(audit_run_id=run_id, state="not-met", finding_ids=["P17-AUD-001"])],
            findings=[finding_record(audit_run_id=run_id, observed_at=audit_head)],
        )
        self.write_audit_index([member])
        if self.commit_live_audit:
            self.commit_paths(self.assurance_dir)
        self.source_head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        self.plan, self.dispositions = self.write_plan()
        for path in (self.plan, self.dispositions):
            path.write_text(path.read_text(encoding="utf-8").replace(REMEDIATION_HEAD, self.source_head), encoding="utf-8")
        self.baseline = self.receipts / "baseline.json"
        self.tested = self.receipts / "tested.json"

    def commit_paths(self, *paths: Path) -> None:
        for args in (("add", "--", *(p.relative_to(self.root).as_posix() for p in paths)), ("commit", "-m", "test: source proof fixture")):
            result = run_git(self.root, *args)
            self.assertEqual(0, result.returncode, result.stderr)

    def proof(self, *args: str | Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run([sys.executable, str(HELPER), *(str(arg) for arg in args)], cwd=self.root, capture_output=True, text=True, check=False)

    def digest(self) -> str:
        return hashlib.sha256(self.plan.read_bytes() + b"\0" + self.dispositions.read_bytes()).hexdigest()

    def capture_baseline(self) -> None:
        result = self.proof("baseline", self.plan, "--plan-sha256", self.digest(), "--output", self.baseline)
        self.assertEqual(0, result.returncode, result.stderr)

    def capture_tested(self) -> None:
        result = self.proof("tested", self.baseline, "--baseline-sha256", sha256(self.baseline), "--output", self.tested)
        self.assertEqual(0, result.returncode, result.stderr)

    def write_applied_result(self) -> Path:
        result, _ = self.write_result(self.plan, self.dispositions)
        text = result.read_text(encoding="utf-8")
        if "**Outcome**:" not in text:
            result.write_text(text + "**Outcome**: APPLIED\n", encoding="utf-8")
        return result

    def verify(self, result: Path, *args: str) -> subprocess.CompletedProcess[str]:
        return self.proof("verify", result, "--baseline", self.baseline, "--baseline-sha256", sha256(self.baseline), "--tested", self.tested, "--tested-sha256", sha256(self.tested), *args)

    def prepare(self) -> Path:
        self.capture_baseline()
        self.source.write_text("repaired source\n", encoding="utf-8")
        self.capture_tested()
        return self.write_applied_result()

    def stage_result(self, result: Path) -> None:
        paths = [self.source, self.plan, self.dispositions, result, result.with_name("remediation.result.dispositions.jsonl")]
        staged = run_git(self.root, "add", "--", *(p.relative_to(self.root).as_posix() for p in paths))
        self.assertEqual(0, staged.returncode, staged.stderr)


class SourceProofTests(SourceProofFixture):
    def test_worktree_receipts_preserve_unrelated_dirty_and_untracked_files(self) -> None:
        self.research.write_text("carried in research\n", encoding="utf-8")
        unrelated = self.root / "untracked notes.txt"
        unrelated.write_text("keep me\n", encoding="utf-8")
        result = self.prepare()
        checked = self.verify(result)
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.assertEqual("carried in research\n", self.research.read_text(encoding="utf-8"))
        self.assertEqual("keep me\n", unrelated.read_text(encoding="utf-8"))
        self.assertEqual(self.source_head, run_git(self.root, "rev-parse", "HEAD").stdout.strip())

    def test_actual_unreported_change_is_rejected(self) -> None:
        self.capture_baseline()
        self.source.write_text("repaired source\n", encoding="utf-8")
        self.research.write_text("unexpected modification\n", encoding="utf-8")
        self.capture_tested()
        checked = self.verify(self.write_applied_result())
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("unattributed", checked.stderr)

    def test_recorded_but_unchanged_path_is_rejected(self) -> None:
        self.capture_baseline()
        self.capture_tested()
        checked = self.verify(self.write_applied_result())
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("not changed", checked.stderr)

    def test_source_change_after_tested_capture_is_rejected(self) -> None:
        result = self.prepare()
        self.source.write_text("later untested edit\n", encoding="utf-8")
        checked = self.verify(result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("tested input", checked.stderr)

    def test_carried_in_content_change_is_rejected_before_tested_receipt(self) -> None:
        self.research.write_text("carried in\n", encoding="utf-8")
        self.capture_baseline()
        self.research.write_text("changed carried in\n", encoding="utf-8")
        checked = self.proof("tested", self.baseline, "--baseline-sha256", sha256(self.baseline), "--output", self.tested)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("carried-in", checked.stderr)
        self.assertFalse(self.tested.exists())

    def test_wrong_receipt_digest_is_rejected(self) -> None:
        self.capture_baseline()
        checked = self.proof("tested", self.baseline, "--baseline-sha256", "0" * 64, "--output", self.tested)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("digest", checked.stderr)
        self.assertFalse(self.tested.exists())

    def test_receipt_outputs_must_be_external_and_new(self) -> None:
        inside = self.root / "receipt.json"
        checked = self.proof("baseline", self.plan, "--plan-sha256", self.digest(), "--output", inside)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("outside", checked.stderr)
        self.assertFalse(inside.exists())
        self.baseline.write_bytes(b"preserve")
        checked = self.proof("baseline", self.plan, "--plan-sha256", self.digest(), "--output", self.baseline)
        self.assertNotEqual(0, checked.returncode)
        self.assertEqual(b"preserve", self.baseline.read_bytes())

    def test_baseline_rejects_owned_path_with_carried_in_changes(self) -> None:
        self.source.write_text("unowned preexisting edit\n", encoding="utf-8")
        checked = self.proof("baseline", self.plan, "--plan-sha256", self.digest(), "--output", self.baseline)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("overlap", checked.stderr)

    def test_invalid_apply_digest_cannot_capture_baseline(self) -> None:
        checked = self.proof("baseline", self.plan, "--plan-sha256", "0" * 64, "--output", self.baseline)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("plan_sha256", checked.stderr)
        self.assertFalse(self.baseline.exists())


    def test_new_and_deleted_source_files_are_bound(self) -> None:
        created = self.root / "crates/opi-agent/src/new file.rs"
        records = [json.loads(line) for line in self.dispositions.read_text(encoding="utf-8").splitlines()]
        records[0]["changed_paths"].append(created.relative_to(self.root).as_posix())
        write_jsonl(self.dispositions, records)
        self.capture_baseline()
        self.source.unlink()
        created.write_text("replacement\n", encoding="utf-8")
        self.capture_tested()
        checked = self.verify(self.write_applied_result())
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_index_requires_exact_tested_blobs(self) -> None:
        result = self.prepare()
        self.stage_result(result)
        checked = self.verify(result, "--stage", "index")
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.source.write_text("unstaged change\n", encoding="utf-8")
        checked = self.verify(result, "--stage", "index")
        self.assertNotEqual(0, checked.returncode)

    def test_stale_staged_blob_is_rejected(self) -> None:
        self.capture_baseline()
        self.source.write_text("staged old edit\n", encoding="utf-8")
        run_git(self.root, "add", "--", self.source.relative_to(self.root).as_posix())
        self.source.write_text("tested final edit\n", encoding="utf-8")
        self.capture_tested()
        result = self.write_applied_result()
        run_git(self.root, "add", "--", self.assurance_dir.relative_to(self.root).as_posix())
        checked = self.verify(result, "--stage", "index")
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("index", checked.stderr)

    def test_unrelated_staged_file_cannot_enter_commit(self) -> None:
        self.research.write_text("carried in research\n", encoding="utf-8")
        run_git(self.root, "add", "--", self.research.name)
        result = self.prepare()
        self.stage_result(result)
        checked = self.verify(result, "--stage", "index")
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("unrelated staged", checked.stderr)

    def test_postcommit_checks_parent_tree_and_preserved_research(self) -> None:
        self.research.write_text("carried in research\n", encoding="utf-8")
        result = self.prepare()
        self.stage_result(result)
        committed = run_git(self.root, "commit", "-m", "fix: fixture repair")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        checked = self.verify(result, "--stage", "commit", "--commit", head)
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.assertEqual("carried in research\n", self.research.read_text(encoding="utf-8"))

    def test_postcommit_rejects_additional_commit(self) -> None:
        result = self.prepare()
        self.stage_result(result)
        run_git(self.root, "commit", "-m", "fix: fixture repair")
        run_git(self.root, "commit", "--allow-empty", "-m", "test: unrelated commit")
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        checked = self.verify(result, "--stage", "commit", "--commit", head)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("parent", checked.stderr)

    def test_changed_plan_bytes_invalidate_receipts(self) -> None:
        result = self.prepare()
        with self.plan.open("a", encoding="utf-8") as handle:
            handle.write("\nChanged plan\n")
        checked = self.verify(result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("plan", checked.stderr)

    def test_wrong_tested_digest_is_rejected(self) -> None:
        result = self.prepare()
        checked = self.proof("verify", result, "--baseline", self.baseline, "--baseline-sha256", sha256(self.baseline), "--tested", self.tested, "--tested-sha256", "0" * 64)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("digest", checked.stderr)

    def test_changed_symlink_mode_is_rejected_without_following_target(self) -> None:
        self.capture_baseline()
        self.source.write_text("../../../../outside-secret", encoding="utf-8")
        blob = subprocess.run(["git", "-C", str(self.root), "hash-object", "-w", "--stdin"], input=b"../../../../outside-secret", capture_output=True, check=True).stdout.decode().strip()
        changed = run_git(self.root, "update-index", "--cacheinfo", "120000", blob, self.source.relative_to(self.root).as_posix())
        self.assertEqual(0, changed.returncode, changed.stderr)
        self.capture_tested()
        checked = self.verify(self.write_applied_result())
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("changed symlink", checked.stderr)

    def test_mode_only_change_is_bound_to_the_index(self) -> None:
        run_git(self.root, "config", "core.filemode", "false")
        self.capture_baseline()
        changed = run_git(self.root, "update-index", "--chmod=+x", self.source.relative_to(self.root).as_posix())
        self.assertEqual(0, changed.returncode, changed.stderr)
        self.capture_tested()
        result = self.write_applied_result()
        run_git(self.root, "add", "--", self.assurance_dir.relative_to(self.root).as_posix())
        checked = self.verify(result, "--stage", "index")
        self.assertEqual(0, checked.returncode, checked.stderr)
        run_git(self.root, "update-index", "--chmod=-x", self.source.relative_to(self.root).as_posix())
        checked = self.verify(result, "--stage", "index")
        self.assertNotEqual(0, checked.returncode)

    def test_pending_assurance_transaction_is_preserved_and_rejected(self) -> None:
        pending = self.root / ".git/opi-assurance-transactions/phase17-test"
        pending.mkdir(parents=True)
        sentinel = pending / "journal.json"
        sentinel.write_bytes(b"must be recovered by owner")
        checked = self.proof("baseline", self.plan, "--plan-sha256", self.digest(), "--output", self.baseline)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("pending assurance transaction", checked.stderr)
        self.assertEqual(b"must be recovered by owner", sentinel.read_bytes())
        self.assertFalse(self.baseline.exists())

    def test_repository_paths_reject_dot_and_windows_drive_forms(self) -> None:
        for name in (".", "C:/outside/file", "C:relative-drive-file", "../outside"):
            with self.subTest(name=name):
                with self.assertRaises(ProofError):
                    repo_path(name)

    def test_unowned_index_only_change_is_rejected_in_worktree_verification(self) -> None:
        result = self.prepare()
        original = self.research.read_bytes()
        self.research.write_text("unexpected staged content\n", encoding="utf-8")
        run_git(self.root, "add", "--", self.research.name)
        self.research.write_bytes(original)
        checked = self.verify(result)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("unowned index", checked.stderr)

    def test_unowned_index_only_change_is_rejected_before_tested_capture(self) -> None:
        self.capture_baseline()
        self.source.write_text("repaired source\n", encoding="utf-8")
        original = self.research.read_bytes()
        self.research.write_text("unexpected staged content\n", encoding="utf-8")
        run_git(self.root, "add", "--", self.research.name)
        self.research.write_bytes(original)
        checked = self.proof("tested", self.baseline, "--baseline-sha256", sha256(self.baseline), "--output", self.tested)
        self.assertNotEqual(0, checked.returncode)
        self.assertIn("unowned index", checked.stderr)
        self.assertFalse(self.tested.exists())

    def test_index_race_after_snapshot_is_rejected_before_baseline_write(self) -> None:
        original_validate = proof_module.validator.validate_apply
        validations = 0

        def change_after_validation(plan: Path, approved_digest: str) -> list[str]:
            nonlocal validations
            errors = original_validate(plan, approved_digest)
            validations += 1
            if validations == 2:
                original = self.research.read_bytes()
                self.research.write_text("racing index edit\n", encoding="utf-8")
                run_git(self.root, "add", "--", self.research.name)
                self.research.write_bytes(original)
            return errors

        with patch.object(proof_module.validator, "validate_apply", side_effect=change_after_validation):
            with self.assertRaisesRegex(ProofError, "index changed"):
                proof_module.capture_baseline(Namespace(path=self.plan, plan_sha256=self.digest(), output=self.baseline))
        self.assertFalse(self.baseline.exists())

    def test_owned_edit_and_stage_after_tested_comparison_is_rejected(self) -> None:
        result = self.prepare()
        self.stage_result(result)
        original_index = proof_module.index_entries
        original_snapshot = proof_module.stable_snapshot
        compared = False
        injected = False

        def snapshot_then_arm(repository: Path, exempt: set[str]) -> dict[str, object]:
            nonlocal compared
            state = original_snapshot(repository, exempt)
            compared = True
            return state

        def change_before_target(repository: Path) -> dict[str, dict[str, str]]:
            nonlocal injected
            if compared and not injected:
                injected = True
                self.source.write_text("untested race edit\n", encoding="utf-8")
                run_git(self.root, "add", "--", self.source.relative_to(self.root).as_posix())
            return original_index(repository)

        with patch.object(proof_module, "index_entries", side_effect=change_before_target), patch.object(proof_module, "stable_snapshot", side_effect=snapshot_then_arm):
            with self.assertRaisesRegex(ProofError, "tested"):
                proof_module.verify(Namespace(
                    path=result, baseline=self.baseline, baseline_sha256=sha256(self.baseline),
                    tested=self.tested, tested_sha256=sha256(self.tested), stage="index", commit=None,
                ))
        self.assertTrue(injected)


class UncommittedAuditProofTests(SourceProofFixture):
    commit_live_audit = False

    def test_uncommitted_live_audit_can_materialize_with_exact_repair(self) -> None:
        result = self.prepare()
        audited = {
            path.name: path.read_bytes()
            for path in self.assurance_dir.iterdir()
            if path.name.startswith("audit.")
        }
        added = run_git(self.root, "add", "--", self.source.relative_to(self.root).as_posix(), self.assurance_dir.relative_to(self.root).as_posix())
        self.assertEqual(0, added.returncode, added.stderr)
        checked = self.verify(result, "--stage", "index")
        self.assertEqual(0, checked.returncode, checked.stderr)
        committed = run_git(self.root, "commit", "-m", "fix: materialize bound audit and repair")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        checked = self.verify(result, "--stage", "commit", "--commit", head)
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.assertEqual(audited, {name: (self.assurance_dir / name).read_bytes() for name in audited})

    def test_frozen_uncommitted_member_cannot_change_after_baseline(self) -> None:
        self.capture_baseline()
        report = self.assurance_dir / "audit.codex.gpt56.md"
        with report.open("a", encoding="utf-8") as handle:
            handle.write("\nChanged observation\n")
        checked = self.proof("tested", self.baseline, "--baseline-sha256", sha256(self.baseline), "--output", self.tested)
        self.assertNotEqual(0, checked.returncode)
        self.assertFalse(self.tested.exists())


if __name__ == "__main__":
    import unittest

    unittest.main()
