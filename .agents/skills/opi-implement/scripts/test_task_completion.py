"""Offline source/commit contract tests; every repository is an isolated fixture."""

import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("task_completion.py")


class CompletionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        self.git("init", "-q")
        self.git("config", "user.name", "Fixture")
        self.git("config", "user.email", "fixture@example.invalid")
        self.git("config", "core.autocrlf", "false")
        self.write("src/main.rs", "old\n")
        self.write("docs/spec.md", "# Contract\n")
        self.write("outside.txt", "base\n")
        self.write(".opi-impl-state.json", '{"schema_version":2}\n')
        self.git("add", "src/main.rs", "docs/spec.md", "outside.txt", ".opi-impl-state.json")
        self.git("commit", "-qm", "fixture baseline")
        self.start = self.git("rev-parse", "HEAD").strip()
        self.task = {"id": "20.1", "definition_of_done": "Exact DoD\nwith Unicode 中文",
                     "start_commit": self.start, "task_owned_paths": ["src/**"],
                     "evaluator_required": True, "acceptance_scenarios": []}
        self.task_path = self.json_file("task.json", self.task)
        self.baseline = self.root / "baseline.json"
        self.capture = self.root / "capture.json"
        self.review = self.root / "review.json"

    def write(self, name, text):
        file = self.repo / name
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text(text, encoding="utf-8", newline="\n")
        return file

    def json_file(self, name, value):
        file = self.root / name
        file.write_text(json.dumps(value, ensure_ascii=False), encoding="utf-8")
        return file

    def git(self, *args):
        result = subprocess.run(["git", "-C", str(self.repo), *args], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def run_cli(self, *args, ok=True):
        result = subprocess.run([sys.executable, "-B", str(SCRIPT), *map(str, args)],
                                cwd=self.repo, capture_output=True, text=True)
        if ok:
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def digest(self, path):
        return hashlib.sha256(path.read_bytes()).hexdigest()

    def baseline_args(self):
        return ["baseline", "--task", self.task_path, "--source-design-path", "docs/spec.md",
                "--output", self.baseline]

    def prepare(self, *, mode="worktree", footer_change=None, rust=None):
        self.run_cli(*self.baseline_args())
        self.write("src/main.rs", "new\n")
        self.write("src/new.rs", "untracked implementation\n")
        self.footers = {"Opi-Task": self.task["id"],
                        "Opi-DoD-SHA256": hashlib.sha256(self.task["definition_of_done"].encode()).hexdigest(),
                        "Opi-Verification": "library; fixture test passed", "Opi-Evaluator": "passed"}
        if footer_change:
            self.footers.update(footer_change)
        self.footer_path = self.json_file("footers.json", self.footers)
        if mode == "commit":
            self.commit()
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", mode, "--output", self.capture)
        stdout = self.root / "stdout.log"
        stderr = self.root / "stderr.log"
        stdout.write_text("observed success\n", encoding="utf-8")
        stderr.write_text("", encoding="utf-8")
        check = {"command": "fixture command", "exit_code": 0,
                 "stdout": str(stdout), "stderr": str(stderr), "kind": "gate"}
        if rust:
            check.update(command="cargo test -p fixture --lib behavior -- --exact",
                         kind="behavioral", rust=rust, stdout=rust["run_output"])
        self.proof = self.json_file("proof.json", {"checks": [check]})

    def seal(self, ok=True):
        result = self.run_cli("review", self.capture, "--sha256", self.digest(self.capture),
                             "--evidence", self.proof, "--footers", self.footer_path,
                             "--output", self.review, ok=ok)
        if ok:
            self.result_path = self.json_file("workflow.json", {
                "review_sha256": self.digest(self.review),
                "candidate_sha256": self.digest(self.capture), "must_fix": [],
                "flagged_for_human": [], "rejected": [], "wf_ref": "fixture-runtime-id",
                "reviewed_input": json.loads(self.review.read_text(encoding="utf-8")),
                "report": {"summary": "reviewed", "must_fix": [], "flagged_for_human": [], "rejected": []}})
        return result

    def verify(self, stage="worktree", *, ok=True, ledger=None):
        args = ["verify", self.review, "--sha256", self.digest(self.review), "--stage", stage,
                "--workflow-result", self.result_path]
        if ledger:
            args += ["--ledger", ledger]
        return self.run_cli(*args, ok=ok)

    def commit(self):
        self.git("add", "src/main.rs", "src/new.rs")
        body = "fixture implementation\n\n" + "\n".join(k + ": " + v for k, v in self.footers.items())
        self.git("commit", "-qm", body)
        return self.git("rev-parse", "HEAD").strip()

    def test_full_worktree_index_commit_checkpoint(self):
        self.prepare()
        self.seal()
        data = json.loads(self.review.read_text(encoding="utf-8"))
        self.assertEqual({row["path"] for row in data["files"]}, {"src/main.rs", "src/new.rs"})
        self.verify()
        self.git("add", "src/main.rs", "src/new.rs")
        self.verify("index")
        commit = self.commit()
        self.verify("commit")
        task = dict(self.task, status="passing", verified_at_commit=commit, evidence=self.footers)
        ledger = self.json_file("ledger.json", {"schema_version": 2, "tasks": [task]})
        self.verify("checkpoint", ledger=ledger)

    def test_drift_between_capture_and_evidence_is_rejected(self):
        self.prepare()
        self.write("src/main.rs", "changed after test input\n")
        self.assertIn("source", self.seal(ok=False).stderr)

    def test_untracked_file_added_after_review_is_rejected(self):
        self.prepare(); self.seal()
        self.write("src/missed.rs", "unreviewed\n")
        self.verify(ok=False)

    def test_untracked_file_omitted_from_index_is_rejected(self):
        self.prepare(); self.seal()
        self.git("add", "src/main.rs")
        self.verify("index", ok=False)

    def test_capture_refuses_staged_change_absent_from_working_candidate(self):
        self.prepare()
        self.write("src/main.rs", "staged only\n")
        self.git("add", "src/main.rs")
        self.write("src/main.rs", "old\n")
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--output", self.root / "index-only.json", ok=False)

    def test_unrelated_staged_path_blocks_index(self):
        self.prepare(); self.seal()
        self.write("outside.txt", "unrelated\n")
        self.git("add", "src/main.rs", "src/new.rs", "outside.txt")
        self.verify("index", ok=False)

    def test_bad_dod_footer_rejected_before_review(self):
        self.prepare(footer_change={"Opi-DoD-SHA256": "0" * 64})
        self.seal(ok=False)

    def test_workflow_missing_or_wrong_identity_cannot_pass(self):
        self.prepare(); self.seal()
        value = json.loads(self.result_path.read_text(encoding="utf-8"))
        value["review_sha256"] = "0" * 64
        self.json_file("workflow.json", value)
        self.verify(ok=False)

    def test_workflow_blocker_cannot_pass(self):
        self.prepare(); self.seal()
        value = json.loads(self.result_path.read_text(encoding="utf-8"))
        value["must_fix"] = [{"problem": "unresolved"}]
        self.json_file("workflow.json", value)
        self.verify(ok=False)

    def test_manual_commit_review(self):
        self.prepare(mode="commit"); self.seal()
        self.verify("commit")
        self.assertEqual(json.loads(self.review.read_text(encoding="utf-8"))["mode"], "commit")

    def test_missing_workflow_run_identity_is_rejected(self):
        self.prepare(); self.seal()
        result = json.loads(self.result_path.read_text(encoding="utf-8"))
        result.pop("wf_ref")
        self.json_file("workflow.json", result)
        self.verify(ok=False)

    def test_capture_accepts_documented_append_only_ownership(self):
        self.run_cli(*self.baseline_args())
        self.write("additional.rs", "new owned code\n")
        current = dict(self.task, task_owned_paths=["src/**", "additional.rs"],
                       inference_notes=[{"field": "task_owned_paths", "reason": "required consumer"}])
        task = self.json_file("current-task.json", current)
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--task", task, "--output", self.capture)

    def test_capture_rejects_dod_mutation(self):
        self.run_cli(*self.baseline_args())
        self.write("src/main.rs", "new\n")
        task = self.json_file("current-task.json", dict(self.task, definition_of_done="changed"))
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--task", task, "--output", self.capture, ok=False)

    def test_admitted_documentation_source_edit_binds_final_verification_input(self):
        self.task.update(tier="documentation", definition_of_done="Update docs/spec.md with the reviewed clarification.",
                         task_owned_paths=["src/**", "docs/spec.md"])
        self.json_file("task.json", self.task)
        self.prepare()
        original = json.loads(self.baseline.read_text(encoding="utf-8"))["sources"]
        source = self.write("docs/spec.md", "# Contract\nReviewed clarification.\n")
        self.capture = self.root / "source-candidate.json"
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--output", self.capture)
        self.seal(); self.verify()
        reviewed = json.loads(self.review.read_text(encoding="utf-8"))
        self.assertEqual(reviewed["baseline_sources"], original)
        self.assertEqual(reviewed["source_sha256"], self.digest(source))
        self.assertNotEqual(reviewed["sources"], original)
        self.git("add", "src/main.rs", "src/new.rs", "docs/spec.md")
        self.verify("index")
        commit = self.commit()
        self.verify("commit")
        task = dict(self.task, status="passing", verified_at_commit=commit, evidence=self.footers)
        ledger = self.json_file("ledger.json", {"schema_version": 2, "tasks": [task]})
        self.verify("checkpoint", ledger=ledger)

    def test_manual_source_edit_recovery_preserves_original_authority(self):
        self.task.update(tier="documentation", definition_of_done="Update docs/spec.md with the reviewed clarification.",
                         task_owned_paths=["src/**", "docs/spec.md"])
        self.json_file("task.json", self.task)
        self.prepare()
        original = json.loads(self.baseline.read_text(encoding="utf-8"))["sources"]
        self.write("docs/spec.md", "# Contract\nReviewed clarification.\n")
        self.git("add", "docs/spec.md")
        self.commit()
        self.baseline = self.root / "recovered-baseline.json"
        self.run_cli(*self.baseline_args(), "--mode", "commit")
        self.assertEqual(json.loads(self.baseline.read_text(encoding="utf-8"))["sources"], original)
        self.capture = self.root / "recovered-candidate.json"
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "commit", "--output", self.capture)
        self.seal(); self.verify("commit")

    def test_registered_source_edit_requires_original_documentation_declaration(self):
        variants = [
            {"tier": "library", "task_owned_paths": ["src/**", "docs/spec.md"],
             "definition_of_done": "Update docs/spec.md."},
            {"tier": "documentation", "task_owned_paths": ["src/**", "docs/**"],
             "definition_of_done": "Update docs/spec.md."},
            {"tier": "documentation", "task_owned_paths": ["src/**", "docs/spec.md"],
             "definition_of_done": "Update the unrelated guide."},
        ]
        for index, fields in enumerate(variants):
            with self.subTest(fields=fields):
                self.json_file("task.json", dict(self.task, **fields))
                self.baseline = self.root / f"baseline-source-{index}.json"
                self.run_cli(*self.baseline_args())
                self.write("docs/spec.md", "# Unauthorized source edit\n")
                result = self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                                      "--mode", "worktree", "--output", self.capture, ok=False)
                self.assertIn("outside admitted documentation scope", result.stderr)
                self.write("docs/spec.md", "# Contract\n")

    def test_source_ownership_append_cannot_expand_original_admission(self):
        self.task.update(tier="documentation", definition_of_done="Update docs/spec.md.")
        self.json_file("task.json", self.task)
        self.run_cli(*self.baseline_args())
        latest = self.json_file("latest.json", dict(self.task, task_owned_paths=["src/**", "docs/spec.md"],
                                inference_notes=[{"field": "task_owned_paths", "reason": "source required"}]))
        self.write("docs/spec.md", "# Unauthorized source edit\n")
        result = self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                              "--mode", "worktree", "--task", latest, "--output", self.capture, ok=False)
        self.assertIn("outside admitted documentation scope", result.stderr)

    def test_admitted_source_drift_after_capture_and_review_is_rejected(self):
        self.task.update(tier="documentation", definition_of_done="Update docs/spec.md.",
                         task_owned_paths=["src/**", "docs/spec.md"])
        self.json_file("task.json", self.task)
        self.prepare()
        self.write("docs/spec.md", "# Captured source\n")
        self.capture = self.root / "source-candidate.json"
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--output", self.capture)
        self.write("docs/spec.md", "# Unverified source\n")
        self.assertIn("registered source changed", self.seal(ok=False).stderr)
        self.write("docs/spec.md", "# Captured source\n")
        self.seal(); self.verify()
        self.write("docs/spec.md", "# Unreviewed source\n")
        self.assertIn("registered source changed", self.verify(ok=False).stderr)

    def test_normative_source_edit_requires_declared_bilingual_scope(self):
        self.write("docs/opi-spec.md", "# Parent source\n")
        self.write("docs/opi-spec.zh.md", "# 中文来源\n")
        self.git("add", "docs/opi-spec.md", "docs/opi-spec.zh.md")
        self.git("commit", "-qm", "fixture registered parent")
        self.task.update(start_commit=self.git("rev-parse", "HEAD").strip(), tier="documentation")
        variants = [
            (["src/**", "docs/opi-spec.md"], "Update docs/opi-spec.md and docs/opi-spec.zh.md.", False),
            (["src/**", "docs/opi-spec.md", "docs/opi-spec.zh.md"], "Update docs/opi-spec.md.", False),
            (["src/**", "docs/opi-spec.md", "docs/opi-spec.zh.md"], "Update docs/opi-spec.md and docs/opi-spec.zh.md.", True),
        ]
        for index, (paths, dod, accepted) in enumerate(variants):
            with self.subTest(paths=paths, dod=dod):
                self.json_file("task.json", dict(self.task, task_owned_paths=paths, definition_of_done=dod))
                self.baseline = self.root / f"baseline-bilingual-{index}.json"
                self.run_cli(*self.baseline_args(), "--source", "docs/opi-spec.md", "--source", "docs/opi-spec.zh.md")
                self.write("docs/opi-spec.md", "# Updated parent source\n")
                self.write("docs/opi-spec.zh.md", "# 已更新中文来源\n")
                result = self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                                      "--mode", "worktree", "--output", self.capture, ok=accepted)
                if not accepted:
                    self.assertIn("outside admitted documentation scope", result.stderr)
                self.write("docs/opi-spec.md", "# Parent source\n")
                self.write("docs/opi-spec.zh.md", "# 中文来源\n")

    def test_manual_commit_cannot_redefine_its_source_authority(self):
        self.task.update(task_owned_paths=["src/**", "docs/spec.md"])
        self.json_file("task.json", self.task)
        self.prepare()
        self.write("docs/spec.md", "# Unauthorized manual source\n")
        self.git("add", "docs/spec.md")
        self.commit()
        self.baseline = self.root / "manual-baseline.json"
        self.run_cli(*self.baseline_args(), "--mode", "commit")
        self.capture = self.root / "manual-candidate.json"
        result = self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                              "--mode", "commit", "--output", self.capture, ok=False)
        self.assertIn("outside admitted documentation scope", result.stderr)

    def test_manual_resume_can_begin_after_task_commit(self):
        self.prepare(mode="commit")
        # A fresh verifier has the committed candidate, not the old receipt.
        recovered = self.root / "recovered-baseline.json"
        self.run_cli("baseline", "--task", self.task_path, "--source-design-path", "docs/spec.md",
                     "--mode", "commit", "--output", recovered)
        captured = self.root / "recovered-capture.json"
        self.run_cli("capture", recovered, "--sha256", self.digest(recovered),
                     "--mode", "commit", "--output", captured)
        self.capture = captured
        self.seal(); self.verify("commit")

    def test_wrong_actual_footer_is_rejected(self):
        self.prepare(); self.seal()
        self.footers["Opi-Task"] = "wrong"
        self.commit()
        self.verify("commit", ok=False)

    def test_wrong_checkpoint_mirror_is_rejected(self):
        self.prepare(); self.seal()
        commit = self.commit()
        task = dict(self.task, status="passing", verified_at_commit=commit,
                    evidence=dict(self.footers, **{"Opi-Task": "wrong"}))
        ledger = self.json_file("ledger.json", {"schema_version": 2, "tasks": [task]})
        self.verify("checkpoint", ok=False, ledger=ledger)

    def test_checkpoint_cannot_change_other_task_contract(self):
        self.task["verification"] = {"library_gates": "original gate"}
        self.json_file("task.json", self.task)
        self.prepare(); self.seal()
        commit = self.commit()
        task = dict(self.task, status="passing", verified_at_commit=commit, evidence=self.footers,
                    verification={"library_gates": "different gate"})
        ledger = self.json_file("ledger.json", {"schema_version": 2, "tasks": [task]})
        self.verify("checkpoint", ok=False, ledger=ledger)

    def test_capture_cannot_erase_old_session_notes(self):
        self.task["session_notes"] = [{"summary": "retained evidence"}]
        self.json_file("task.json", self.task)
        self.run_cli(*self.baseline_args())
        self.write("src/main.rs", "new\n")
        latest = self.json_file("latest.json", dict(self.task, session_notes=[]))
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--task", latest, "--output", self.capture, ok=False)

    def test_workflow_incomplete_object_cannot_pass(self):
        self.prepare(); self.seal()
        result = json.loads(self.result_path.read_text(encoding="utf-8"))
        result["status"] = "incomplete"
        self.json_file("workflow.json", result)
        self.verify(ok=False)

    def test_workflow_cannot_drop_reviewed_files_while_preserving_digests(self):
        self.prepare(); self.seal()
        result = json.loads(self.result_path.read_text(encoding="utf-8"))
        result["reviewed_input"]["files"] = result["reviewed_input"]["files"][:1]
        self.json_file("workflow.json", result)
        self.verify(ok=False)

    def test_workflow_rejected_finding_cannot_be_malformed(self):
        self.prepare(); self.seal()
        result = json.loads(self.result_path.read_text(encoding="utf-8"))
        result["rejected"] = result["report"]["rejected"] = [{"finding": None, "reason": "looks fine"}]
        self.json_file("workflow.json", result)
        self.verify(ok=False)

    def test_source_and_raw_output_drift_are_rejected(self):
        self.prepare(); self.seal()
        self.write("docs/spec.md", "changed source\n")
        self.verify(ok=False)
        self.write("docs/spec.md", "# Contract\n")
        (self.root / "stdout.log").write_text("changed evidence", encoding="utf-8")
        self.verify(ok=False)

    def test_baseline_carried_in_work_is_preserved(self):
        self.write("outside.txt", "carried in\n")
        self.prepare(); self.seal(); self.verify()
        self.write("outside.txt", "concurrent drift\n")
        self.verify(ok=False)

    def test_receipt_cannot_be_overwritten(self):
        self.run_cli(*self.baseline_args())
        self.run_cli(*self.baseline_args(), ok=False)

    def test_zero_test_execution_cannot_be_behavior_proof(self):
        all_log = self.root / "all.log"
        ignored = self.root / "ignored.log"
        run = self.root / "run.log"
        all_log.write_text("behavior: test\n1 test, 0 benchmarks\n")
        ignored.write_text("0 tests, 0 benchmarks\n")
        run.write_text("test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\n")
        self.prepare(rust={"list_output": str(all_log), "ignored_list_output": str(ignored),
                           "run_output": str(run), "filter": "behavior", "exact": True,
                           "selection": "normal", "expected_count": 1,
                           "list_exit_code": 0, "ignored_list_exit_code": 0})
        self.seal(ok=False)
        proof = json.loads(self.proof.read_text(encoding="utf-8"))
        proof["checks"][0]["command"] = "python scripts/opi-cargo-cache.py run --target /tmp/cache -- test -p fixture --lib behavior -- --exact"
        self.json_file("proof.json", proof)
        self.seal(ok=False)

    def test_toolchain_cargo_test_requires_execution_proof(self):
        self.prepare()
        proof = json.loads(self.proof.read_text(encoding="utf-8"))
        proof["checks"][0].update(command="cargo +stable --color never test -p fixture stale_filter", kind="behavioral")
        self.json_file("proof.json", proof)
        self.seal(ok=False)

    def test_wrapped_cargo_test_requires_execution_proof(self):
        self.prepare()
        (self.root / "stdout.log").write_text(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s\n",
            encoding="utf-8")
        commands = [
            "python scripts/opi-cargo-cache.py run --target /tmp/cache -- test -p fixture stale_filter",
            "python3 -B /mnt/d/repo/scripts/opi-cargo-cache.py run --target=/tmp/cache test -p fixture stale_filter",
            'python "D:\\repo with space\\scripts\\opi-cargo-cache.py" run --target "D:\\Cargo Cache" -- test -p fixture stale_filter',
            "py -3 'D:\\repo\\scripts\\opi-cargo-cache.py' 'run' --target='D:\\Cargo Cache' -- test -p fixture stale_filter",
            '"D:\\Rust Tools\\cargo.exe" +stable --color never test -p fixture stale_filter',
            "'/usr/bin/cargo' test -p fixture stale_filter",
            "python scripts/OPI-CARGO-CACHE.PY run --target C:/cache -- test -p fixture stale_filter",
            "CARGO.EXE test -p fixture stale_filter",
        ]
        for index, command in enumerate(commands):
            with self.subTest(command=command):
                self.review = self.root / f"review-command-{index}.json"
                proof = json.loads(self.proof.read_text(encoding="utf-8"))
                proof["checks"][0].update(command=command, kind="behavioral")
                self.json_file("proof.json", proof)
                self.assertIn("requires Rust execution proof", self.seal(ok=False).stderr)

    def test_real_rust_execution_inventory_is_accepted(self):
        all_log, ignored, run = [self.root / name for name in ("all.log", "ignored.log", "run.log")]
        all_log.write_text("behavior: test\n1 test, 0 benchmarks\n")
        ignored.write_text("0 tests, 0 benchmarks\n")
        run.write_text("test behavior ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n")
        self.prepare(rust={"list_output": str(all_log), "ignored_list_output": str(ignored),
                           "run_output": str(run), "filter": "behavior", "exact": True,
                           "selection": "normal", "expected_count": 1,
                           "list_exit_code": 0, "ignored_list_exit_code": 0})
        self.seal(); self.verify()
        proof = json.loads(self.proof.read_text(encoding="utf-8"))
        proof["checks"][0]["command"] = "python scripts/opi-cargo-cache.py run --target /tmp/cache -- test -p fixture --lib behavior -- --exact"
        self.json_file("proof.json", proof)
        self.review = self.root / "wrapped-review.json"
        self.seal(); self.verify()

    def test_rust_inventory_process_failure_cannot_pass(self):
        all_log, ignored, run = [self.root / name for name in ("all.log", "ignored.log", "run.log")]
        all_log.write_text("behavior: test\n1 test, 0 benchmarks\n")
        ignored.write_text("0 tests, 0 benchmarks\n")
        run.write_text("test behavior ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n")
        self.prepare(rust={"list_output": str(all_log), "ignored_list_output": str(ignored),
                           "run_output": str(run), "filter": "behavior", "exact": True,
                           "list_exit_code": 2, "ignored_list_exit_code": 0})
        self.seal(ok=False)

    def test_deleted_and_renamed_files_are_bound(self):
        self.prepare()
        (self.repo / "src/main.rs").rename(self.repo / "src/renamed.rs")
        self.capture = self.root / "capture-renamed.json"
        self.run_cli("capture", self.baseline, "--sha256", self.digest(self.baseline),
                     "--mode", "worktree", "--output", self.capture)
        self.seal()
        self.git("add", "src/main.rs", "src/new.rs", "src/renamed.rs")
        self.verify("index")

    @unittest.skipUnless(shutil.which("node"), "Node is required for Workflow integration")
    def test_python_receipt_workflow_and_completion_round_trip(self):
        self.prepare(); self.seal()
        harness = SCRIPT.with_name("verify-workflows.tests.js")
        code = """
const fs = require('node:fs');
const {runWorkflow} = require(process.argv[1]);
const candidate = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
runWorkflow('exec', {count: 0, args: {task: candidate.task, sourceDesignPath: candidate.source_design_path,
  candidate, review_sha256: process.argv[3]}}).then(({result}) => process.stdout.write(JSON.stringify(result)))
  .catch(error => {process.stderr.write(String(error));process.exitCode=1});
"""
        run = subprocess.run(["node", "-e", code, str(harness), str(self.review), self.digest(self.review)],
                             capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(run.returncode, 0, run.stderr)
        result = json.loads(run.stdout)
        result["wf_ref"] = "fixture-runtime-id"
        self.json_file("workflow.json", result)
        self.verify()
        # The real Workflow must expose exactly the input it inspected, even if
        # its caller keeps the old digests while dropping a production file.
        altered = code.replace("runWorkflow('exec'", "candidate.files = candidate.files.slice(1);\nrunWorkflow('exec'")
        run = subprocess.run(["node", "-e", altered, str(harness), str(self.review), self.digest(self.review)],
                             capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(run.returncode, 0, run.stderr)
        result = json.loads(run.stdout)
        result["wf_ref"] = "fixture-runtime-id"
        self.json_file("workflow.json", result)
        self.verify(ok=False)


if __name__ == "__main__":
    unittest.main()
