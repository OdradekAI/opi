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


class SourceCheckpointProofTests(SourceProofFixture):
    def checkpoint_tested(
        self, output: Path, predecessor: Path | None = None, predecessor_digest: str | None = None,
    ) -> subprocess.CompletedProcess[str]:
        args: list[str | Path] = [
            "tested", self.baseline, "--baseline-sha256", sha256(self.baseline),
            "--output", output,
        ]
        if predecessor is not None:
            args.extend(("--checkpoint", predecessor, "--checkpoint-sha256", predecessor_digest or sha256(predecessor)))
        return self.proof(*args)

    def capture_checkpoint_tested(self, name: str, predecessor: Path | None = None) -> None:
        self.tested = self.receipts / f"{name}.tested.json"
        checked = self.checkpoint_tested(self.tested, predecessor)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def reject_checkpoint_tested(self, predecessor: Path | None = None, digest: str | None = None) -> None:
        output = self.receipts / "unexpected.tested.json"
        checked = self.checkpoint_tested(output, predecessor, digest)
        self.assertNotEqual(0, checked.returncode, checked.stdout)
        self.assertFalse(output.exists())

    def stage_checkpoint(self) -> None:
        staged = run_git(self.root, "add", "--", self.source.relative_to(self.root).as_posix())
        self.assertEqual(0, staged.returncode, staged.stderr)

    def checkpoint(self, stage: str, output: Path, *args: str | Path) -> subprocess.CompletedProcess[str]:
        return self.proof(
            "checkpoint", self.baseline, "--baseline-sha256", sha256(self.baseline),
            "--tested", self.tested, "--tested-sha256", sha256(self.tested),
            "--stage", stage, "--output", output, *args,
        )

    def reject_checkpoint(self, stage: str, *args: str | Path, reason: str | None = None) -> None:
        output = self.receipts / f"unexpected.{stage}.json"
        checked = self.checkpoint(stage, output, *args)
        self.assertNotEqual(0, checked.returncode, checked.stdout)
        if reason is not None:
            self.assertIn(reason, checked.stderr)
        self.assertFalse(output.exists())

    def seal_checkpoint(self, name: str, predecessor: Path | None = None) -> Path:
        self.capture_checkpoint_tested(name, predecessor)
        self.stage_checkpoint()
        candidate = self.receipts / f"{name}.candidate.json"
        checked = self.checkpoint("index", candidate)
        self.assertEqual(0, checked.returncode, checked.stderr)
        committed = run_git(self.root, "commit", "-m", f"fix: {name} checkpoint")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        receipt = self.receipts / f"{name}.commit.json"
        checked = self.checkpoint(
            "commit", receipt, "--candidate", candidate,
            "--candidate-sha256", sha256(candidate), "--commit", head,
        )
        self.assertEqual(0, checked.returncode, checked.stderr)
        return receipt

    def test_two_source_checkpoints_allow_complete_result_only_final_commit(self) -> None:
        self.capture_baseline()
        self.source.write_text("first repaired source\n", encoding="utf-8")
        first = self.seal_checkpoint("first")
        self.source.write_text("second repaired source\n", encoding="utf-8")
        second = self.seal_checkpoint("second", first)
        self.capture_checkpoint_tested("final", second)
        result = self.write_applied_result()
        checked = self.verify(result)
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.stage_result(result)
        checked = self.verify(result, "--stage", "index")
        self.assertEqual(0, checked.returncode, checked.stderr)
        committed = run_git(self.root, "commit", "-m", "test: final complete result")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        checked = self.verify(result, "--stage", "commit", "--commit", head)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_final_source_edit_after_checkpoint_requires_its_own_tested_input(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        checkpoint = self.seal_checkpoint("first")
        self.source.write_text("final verified source\n", encoding="utf-8")
        self.capture_checkpoint_tested("final", checkpoint)
        result = self.write_applied_result()
        self.stage_result(result)
        checked = self.verify(result, "--stage", "index")
        self.assertEqual(0, checked.returncode, checked.stderr)
        committed = run_git(self.root, "commit", "-m", "fix: final source and complete result")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        checked = self.verify(result, "--stage", "commit", "--commit", head)
        self.assertEqual(0, checked.returncode, checked.stderr)

    def test_checkpoint_does_not_create_or_replace_complete_result(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        checkpoint = self.seal_checkpoint("first")
        self.assertFalse(self.plan.with_name("remediation.result.md").exists())
        self.assertFalse(self.plan.with_name("remediation.result.dispositions.jsonl").exists())
        self.capture_checkpoint_tested("final", checkpoint)
        checked = self.verify(self.plan.with_name("remediation.result.md"))
        self.assertNotEqual(0, checked.returncode)

    def test_later_tested_capture_requires_checkpoint_predecessor(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.seal_checkpoint("first")
        self.reject_checkpoint_tested()

    def test_checkpoint_predecessor_digest_must_match_original_bytes(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        checkpoint = self.seal_checkpoint("first")
        self.reject_checkpoint_tested(checkpoint, "0" * 64)

    def test_earlier_checkpoint_cannot_replace_current_predecessor(self) -> None:
        self.capture_baseline()
        self.source.write_text("first source\n", encoding="utf-8")
        first = self.seal_checkpoint("first")
        self.source.write_text("second source\n", encoding="utf-8")
        self.seal_checkpoint("second", first)
        self.reject_checkpoint_tested(first)

    def test_index_candidate_is_not_a_committed_checkpoint_predecessor(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.seal_checkpoint("first")
        self.reject_checkpoint_tested(self.receipts / "first.candidate.json")

    def test_commit_without_checkpoint_receipt_cannot_advance_tested_input(self) -> None:
        self.capture_baseline()
        self.source.write_text("unreceipted source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        committed = run_git(self.root, "commit", "-m", "fix: unreceipted checkpoint")
        self.assertEqual(0, committed.returncode, committed.stderr)
        self.reject_checkpoint_tested()

    def test_additional_unreceipted_commit_cannot_follow_checkpoint(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        checkpoint = self.seal_checkpoint("first")
        committed = run_git(self.root, "commit", "--allow-empty", "-m", "test: unrelated commit")
        self.assertEqual(0, committed.returncode, committed.stderr)
        self.reject_checkpoint_tested(checkpoint)

    def test_checkpoint_commit_rejects_merge_with_exact_candidate_tree(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        candidate = self.receipts / "first.candidate.json"
        checked = self.checkpoint("index", candidate)
        self.assertEqual(0, checked.returncode, checked.stderr)
        source_tree = run_git(self.root, "rev-parse", "HEAD^{tree}").stdout.strip()
        side = run_git(self.root, "commit-tree", source_tree, "-p", self.source_head, "-m", "test: side commit")
        self.assertEqual(0, side.returncode, side.stderr)
        index_tree = run_git(self.root, "write-tree").stdout.strip()
        merged = run_git(
            self.root, "commit-tree", index_tree, "-p", self.source_head,
            "-p", side.stdout.strip(), "-m", "test: merge with checkpoint tree",
        )
        self.assertEqual(0, merged.returncode, merged.stderr)
        head = merged.stdout.strip()
        moved = run_git(self.root, "update-ref", "HEAD", head, self.source_head)
        self.assertEqual(0, moved.returncode, moved.stderr)
        self.reject_checkpoint(
            "commit", "--candidate", candidate,
            "--candidate-sha256", sha256(candidate), "--commit", head,
        )

    def test_checkpoint_rejects_unrelated_staged_blob_even_when_worktree_matches(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        original = self.research.read_bytes()
        self.research.write_text("unrelated staged content\n", encoding="utf-8")
        staged = run_git(self.root, "add", "--", self.research.name)
        self.assertEqual(0, staged.returncode, staged.stderr)
        self.research.write_bytes(original)
        self.reject_checkpoint("index")

    def test_checkpoint_preserves_but_cannot_publish_carried_in_staged_work(self) -> None:
        self.research.write_text("carried in staged research\n", encoding="utf-8")
        staged = run_git(self.root, "add", "--", self.research.name)
        self.assertEqual(0, staged.returncode, staged.stderr)
        before = run_git(self.root, "ls-files", "--stage", "--", self.research.name).stdout
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        self.reject_checkpoint("index")
        self.assertEqual(before, run_git(self.root, "ls-files", "--stage", "--", self.research.name).stdout)
        self.assertEqual("carried in staged research\n", self.research.read_text(encoding="utf-8"))

    def test_checkpoint_rejects_source_outside_approved_paths(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.research.write_text("unplanned repair\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        self.reject_checkpoint("index")

    def test_checkpoint_rejects_changed_frozen_audit(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        report = self.assurance_dir / "audit.codex.gpt56.md"
        with report.open("a", encoding="utf-8") as handle:
            handle.write("\nChanged observation\n")
        self.reject_checkpoint("index")

    def test_checkpoint_rejects_changed_approved_plan(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        with self.plan.open("a", encoding="utf-8") as handle:
            handle.write("\nChanged plan\n")
        self.reject_checkpoint("index")

    def test_checkpoint_rejects_stale_staged_source(self) -> None:
        self.capture_baseline()
        self.source.write_text("old staged source\n", encoding="utf-8")
        self.stage_checkpoint()
        self.source.write_text("new tested source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.reject_checkpoint("index")

    def test_checkpoint_rejects_staged_repair_reverted_in_tested_worktree(self) -> None:
        other = self.source.with_name("other.rs")
        records = [json.loads(line) for line in self.dispositions.read_text(encoding="utf-8").splitlines()]
        records[0]["changed_paths"].append(other.relative_to(self.root).as_posix())
        write_jsonl(self.dispositions, records)
        self.capture_baseline()
        original = self.source.read_bytes()
        self.source.write_text("staged edit later reverted in worktree\n", encoding="utf-8")
        self.stage_checkpoint()
        self.source.write_bytes(original)
        other.write_text("actual checkpoint repair\n", encoding="utf-8")
        staged = run_git(self.root, "add", "--", other.relative_to(self.root).as_posix())
        self.assertEqual(0, staged.returncode, staged.stderr)
        self.capture_checkpoint_tested("first")
        self.reject_checkpoint("index")

    def test_checkpoint_rejects_source_modified_after_tested_capture(self) -> None:
        self.capture_baseline()
        self.source.write_text("tested source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.source.write_text("untested source\n", encoding="utf-8")
        self.stage_checkpoint()
        self.reject_checkpoint("index")

    def test_checkpoint_cannot_publish_result_outputs_before_complete_verification(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        result = self.write_applied_result()
        self.stage_checkpoint()
        staged = run_git(
            self.root, "add", "--", result.relative_to(self.root).as_posix(),
            result.with_name("remediation.result.dispositions.jsonl").relative_to(self.root).as_posix(),
        )
        self.assertEqual(0, staged.returncode, staged.stderr)
        self.reject_checkpoint("index")

    def test_checkpoint_cannot_publish_bound_plan_pair(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        staged = run_git(
            self.root, "add", "--", self.plan.relative_to(self.root).as_posix(),
            self.dispositions.relative_to(self.root).as_posix(),
        )
        self.assertEqual(0, staged.returncode, staged.stderr)
        self.reject_checkpoint("index")

    def test_checkpoint_commit_must_match_sealed_index_candidate(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        candidate = self.receipts / "first.candidate.json"
        checked = self.checkpoint("index", candidate)
        self.assertEqual(0, checked.returncode, checked.stderr)
        self.source.write_text("different committed source\n", encoding="utf-8")
        self.stage_checkpoint()
        committed = run_git(self.root, "commit", "-m", "fix: changed candidate")
        self.assertEqual(0, committed.returncode, committed.stderr)
        head = run_git(self.root, "rev-parse", "HEAD").stdout.strip()
        self.reject_checkpoint(
            "commit", "--candidate", candidate,
            "--candidate-sha256", sha256(candidate), "--commit", head,
        )

    def test_replace_ref_cannot_mask_unverified_checkpoint_tree(self) -> None:
        self.capture_baseline()
        self.source.write_text("checkpoint source\n", encoding="utf-8")
        self.capture_checkpoint_tested("first")
        self.stage_checkpoint()
        candidate = self.receipts / "first.candidate.json"
        checked = self.checkpoint("index", candidate)
        self.assertEqual(0, checked.returncode, checked.stderr)

        verified_tree = run_git(self.root, "write-tree").stdout.strip()
        verified = run_git(self.root, "commit-tree", verified_tree, "-p", self.source_head, "-m", "test: verified tree")
        self.assertEqual(0, verified.returncode, verified.stderr)
        source_name = self.source.relative_to(self.root).as_posix()
        original_blob = run_git(self.root, "rev-parse", f"{self.source_head}:{source_name}").stdout.strip()
        changed = run_git(self.root, "update-index", "--cacheinfo", f"100644,{original_blob},{source_name}")
        self.assertEqual(0, changed.returncode, changed.stderr)
        wrong_tree = run_git(self.root, "write-tree").stdout.strip()
        wrong = run_git(self.root, "commit-tree", wrong_tree, "-p", self.source_head, "-m", "test: wrong tree")
        self.assertEqual(0, wrong.returncode, wrong.stderr)
        head = wrong.stdout.strip()
        for args in (
            ("read-tree", verified_tree),
            ("update-ref", "HEAD", head, self.source_head),
            ("config", "core.useReplaceRefs", "true"),
            ("replace", head, verified.stdout.strip()),
        ):
            checked = run_git(self.root, *args)
            self.assertEqual(0, checked.returncode, checked.stderr)
        self.assertNotEqual(wrong_tree, verified_tree)
        self.assertEqual("checkpoint source\n", run_git(self.root, "show", f"HEAD:{source_name}").stdout)
        self.reject_checkpoint(
            "commit", "--candidate", candidate, "--candidate-sha256", sha256(candidate),
            "--commit", head, reason="verified index",
        )


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
