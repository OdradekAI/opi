"""Exercise the committed-ledger boundary through the audit reader CLI."""

from __future__ import annotations

import copy
import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / ".agents/skills/opi-audit/scripts/read_audit_scope.py"
SNAPSHOT = "docs/snapshots/phase19/opi-impl-state.json"
SPEC = "docs/opi-spec.md"
SUPPLEMENT = "docs/phase19.md"
HISTORY = "WITHHELD_HISTORICAL_CONCLUSION"


class AuditScopeTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        self.git("init", "--quiet")
        self.git("config", "user.name", "Audit Scope Test")
        self.git("config", "user.email", "audit@example.invalid")
        self.git("config", "core.autocrlf", "false")
        self.git("config", "commit.gpgsign", "false")
        self.git("config", "core.hooksPath", str(self.repo / "no-hooks"))
        self.write(SPEC, b"# Current specification\r\n")
        self.write(SUPPLEMENT, b"# Phase 19 requirements\n")
        self.task = {
            "id": "19.1", "phase": 19, "title": "Native evaluation",
            "definition_of_done": "Grade the selected native trial.",
            "definition_source": "verbatim", "depends_on": [],
            "verification": {
                "library_gates": ["python check.py"], "behavioral_tests": [],
                "snapshot_tests": [], "smoke_addendum": None,
                "gate_results": HISTORY,
            },
            "acceptance_scenarios": [{
                "id": "A1", "source": SUPPLEMENT + "#acceptance",
                "scenario": "The native grader consumes the trial artifact.",
                "verification": ["python check.py"],
                "production_call_sites": ["native::grade"],
                "status": "met", "evidence": HISTORY,
            }],
            "inference_notes": [
                {"field": "reuse_search", "reason": "searched=grader; reused=grade; gap=none",
                 "source": SUPPLEMENT, "evidence": HISTORY},
                {"field": "surface_necessity", "reason": "public_api=none", "source": SUPPLEMENT},
                {"field": "simplification_ceiling", "reason": "ceiling=existing grader", "source": SUPPLEMENT},
                {"field": "execution_metadata", "reason": HISTORY, "source": HISTORY},
                {"field": "unknown_future_note", "reason": HISTORY, "source": HISTORY},
            ],
            "production_call_sites": ["native::grade"],
            "status": "passing", "evidence": {"summary": HISTORY},
            "blocker": HISTORY, "session_notes": [HISTORY],
            "future_field": {"nested": HISTORY},
        }
        self.root_state = {
            "schema_version": 2, "current_phase": 19,
            "spec_files": [SPEC], "spec_files_sha256": {SPEC: "0" * 64},
            "tasks": [],
            "phase_exit": {
                "17": {"evaluator_summary": HISTORY},
                "18": {"evaluator_summary": HISTORY},
                "19": {"snapshot_path": SNAPSHOT, "evaluator_summary": HISTORY,
                       "criteria_trace": [HISTORY], "task_summary": [HISTORY]},
            },
            "session_notes": [HISTORY], "verify_runs": [HISTORY],
            "future_field": HISTORY,
        }
        self.phase_state = copy.deepcopy(self.root_state)
        self.phase_state["tasks"] = [self.task, {"phase": 18, "title": HISTORY}]
        self.phase_state["spec_files"] = [SPEC, SUPPLEMENT]
        self.phase_state["spec_files_sha256"][SUPPLEMENT] = hashlib.sha256(
            (self.repo / SUPPLEMENT).read_bytes()
        ).hexdigest()
        self.commit_states()

    def git(self, *args: str) -> str:
        result = subprocess.run(
            ["git", "-C", str(self.repo), *args], capture_output=True, text=True,
        )
        self.assertEqual(0, result.returncode, result.stderr)
        return result.stdout.strip()

    def write(self, path: str, data: bytes) -> None:
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)

    def commit_states(self) -> None:
        for path, value in [(".opi-impl-state.json", self.root_state), (SNAPSHOT, self.phase_state)]:
            self.write(path, (json.dumps(value, indent=2) + "\r\n").encode())
        self.git("add", ".opi-impl-state.json", SNAPSHOT, SPEC, SUPPLEMENT)
        self.git("commit", "--quiet", "-m", "test fixture")
        self.head = self.git("rev-parse", "HEAD")

    def run_reader(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--phase", "19", *args],
            cwd=self.repo, capture_output=True, text=True,
        )

    def test_projection_keeps_requirements_and_omits_nested_history(self) -> None:
        result = self.run_reader()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertNotIn(HISTORY, result.stdout + result.stderr)
        scope = json.loads(result.stdout)
        self.assertEqual(self.head, scope["audit_head"])
        self.assertEqual({"19": {"snapshot_path": SNAPSHOT}}, scope["root_state"]["phase_exit"])
        tasks = scope["phase_state"]["tasks"]
        self.assertEqual(["19.1"], [task["id"] for task in tasks])
        task = tasks[0]
        self.assertEqual(self.task["definition_of_done"], task["definition_of_done"])
        self.assertEqual(["python check.py"], task["verification"]["library_gates"])
        self.assertEqual("A1", task["acceptance_scenarios"][0]["id"])
        self.assertEqual(["native::grade"], task["production_call_sites"])
        self.assertEqual(
            ["reuse_search", "surface_necessity", "simplification_ceiling"],
            [note["field"] for note in task["inference_notes"]],
        )
        self.assertNotIn("status", task)
        self.assertNotIn("status", task["acceptance_scenarios"][0])

    def test_hashes_full_committed_bytes_including_withheld_fields(self) -> None:
        self.write(".opi-impl-state.json", b"uncommitted invalid data")
        self.write(SPEC, b"uncommitted spec")
        result = self.run_reader("--head", self.head)
        self.assertEqual(0, result.returncode, result.stderr)
        scope = json.loads(result.stdout)
        sources = scope["baseline_sources"]
        self.assertEqual([".opi-impl-state.json", SNAPSHOT, SPEC, SUPPLEMENT], [s["path"] for s in sources])
        for source in sources:
            raw = subprocess.check_output(["git", "-C", str(self.repo), "show", f'{self.head}:{source["path"]}'])
            self.assertEqual(hashlib.sha256(raw).hexdigest(), source["sha256"])
        self.assertEqual(2, len(scope["stored_hash_mismatches"]))
        self.assertEqual(self.head, scope["audit_head"])

    def test_explicit_ancestor_is_used_after_head_advances(self) -> None:
        sealed = self.head
        self.phase_state["tasks"][0]["definition_of_done"] = "New requirement"
        self.commit_states()
        result = self.run_reader("--head", sealed)
        self.assertEqual(0, result.returncode, result.stderr)
        scope = json.loads(result.stdout)
        self.assertEqual(sealed, scope["audit_head"])
        self.assertEqual("Grade the selected native trial.", scope["phase_state"]["tasks"][0]["definition_of_done"])

    def test_normalized_ledger_hashes_do_not_change_raw_baseline_hashes(self) -> None:
        raw = (self.repo / SPEC).read_bytes()
        normalized_hash = hashlib.sha256(raw.replace(b"\r\n", b"\n")).hexdigest()
        for state in (self.root_state, self.phase_state):
            state["spec_files_sha256"][SPEC] = normalized_hash
        self.commit_states()
        result = self.run_reader()
        self.assertEqual(0, result.returncode, result.stderr)
        scope = json.loads(result.stdout)
        self.assertEqual([], scope["stored_hash_mismatches"])
        baseline_hash = next(s["sha256"] for s in scope["baseline_sources"] if s["path"] == SPEC)
        self.assertEqual(hashlib.sha256(raw).hexdigest(), baseline_hash)
        self.assertNotEqual(normalized_hash, baseline_hash)

    def test_frozen_v1_snapshot_remains_readable(self) -> None:
        self.phase_state["schema_version"] = 1
        self.phase_state["archived_phase"] = self.phase_state.pop("current_phase")
        self.phase_state["spec_path"] = SPEC
        self.phase_state["spec_sha256"] = self.phase_state.pop("spec_files_sha256")[SPEC]
        self.phase_state.pop("spec_files")
        self.commit_states()
        result = self.run_reader()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertNotIn(HISTORY, result.stdout + result.stderr)
        scope = json.loads(result.stdout)
        self.assertEqual(19, scope["phase_state"]["archived_phase"])
        self.assertEqual("19.1", scope["phase_state"]["tasks"][0]["id"])

    def test_active_root_tasks_are_projected_without_runtime_status(self) -> None:
        self.root_state["tasks"] = [copy.deepcopy(self.task)]
        self.commit_states()
        result = self.run_reader()
        self.assertEqual(0, result.returncode, result.stderr)
        self.assertNotIn(HISTORY, result.stdout + result.stderr)
        scope = json.loads(result.stdout)
        self.assertEqual(scope["phase_state"]["tasks"], scope["root_state"]["tasks"])

    def test_invalid_scope_fails_without_echoing_input_or_partial_output(self) -> None:
        cases = [
            ("missing registration", lambda: self.root_state["phase_exit"].pop("19")),
            ("wrong snapshot", lambda: self.root_state["phase_exit"]["19"].update(snapshot_path=HISTORY)),
            ("wrong phase", lambda: self.phase_state.update(current_phase=18)),
            ("malformed tasks", lambda: self.phase_state.update(tasks=HISTORY)),
            ("no target tasks", lambda: self.phase_state.update(tasks=[])),
            ("malformed requirement", lambda: self.phase_state["tasks"][0].update(definition_of_done={"summary": HISTORY})),
            ("missing scenario", lambda: self.phase_state["tasks"][0]["acceptance_scenarios"][0].pop("scenario")),
            ("malformed nested verification", lambda: self.phase_state["tasks"][0]["verification"].update(library_gates=[{"summary": HISTORY}])),
            ("malformed allowed note", lambda: self.phase_state["tasks"][0]["inference_notes"][0].update(reason={"summary": HISTORY})),
            ("invalid schema", lambda: self.phase_state.update(schema_version=99)),
            ("invalid stored hash", lambda: self.phase_state["spec_files_sha256"].update({SPEC: HISTORY})),
            ("bad spec path", lambda: self.phase_state.update(spec_files=[SPEC, "../" + HISTORY])),
            ("missing spec", lambda: self.phase_state.update(spec_files=[SPEC, "docs/missing.md"])),
        ]
        original_root, original_phase = copy.deepcopy(self.root_state), copy.deepcopy(self.phase_state)
        for name, mutate in cases:
            with self.subTest(name=name):
                self.root_state, self.phase_state = copy.deepcopy(original_root), copy.deepcopy(original_phase)
                mutate()
                self.commit_states()
                result = self.run_reader()
                self.assertNotEqual(0, result.returncode)
                self.assertEqual("", result.stdout)
                self.assertIn("AUDIT-INCOMPLETE", result.stderr)
                self.assertNotIn(HISTORY, result.stderr)
                self.assertNotIn("Traceback", result.stderr)

    def test_invalid_json_and_head_do_not_leak_raw_diagnostics(self) -> None:
        self.write(SNAPSHOT, ("{" + HISTORY).encode())
        self.git("add", SNAPSHOT)
        self.git("commit", "--quiet", "-m", "invalid fixture")
        for args in [(), ("--head", HISTORY)]:
            with self.subTest(args=args):
                result = self.run_reader(*args)
                self.assertNotEqual(0, result.returncode)
                self.assertEqual("", result.stdout)
                self.assertIn("AUDIT-INCOMPLETE", result.stderr)
                self.assertNotIn(HISTORY, result.stderr)
                self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
