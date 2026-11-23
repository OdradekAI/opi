from __future__ import annotations

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from common import FIXTURE_DIR, REPO_ROOT
from replay_chat_finish_timing import FIXTURE, compare
from replay_chat_finish_consumers import compare as compare_consumers


class ChatFinishObservations(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.paired = json.loads((FIXTURE_DIR / "phase5-chat-terminal-repaired-verified.json").read_bytes())
        cls.consumers = json.loads((FIXTURE_DIR / "phase5-chat-terminal-consumers-repaired.json").read_bytes())
        cls.fixture = json.loads((REPO_ROOT / FIXTURE).read_bytes())

    def setUp(self) -> None:
        self.pi = copy.deepcopy(self.paired["pi_runs"][0]["trace"])
        self.opi = copy.deepcopy(self.paired["opi_runs"][0]["trace"])
        self.durable = copy.deepcopy(self.consumers["durable_runs"][0]["trace"])
        self.agent = copy.deepcopy(self.consumers["agent_runs"][0]["trace"])

    def test_repaired_terminals_retain_explicit_cancellation_difference(self) -> None:
        self.assertEqual(compare(self.pi,self.opi,self.fixture), self.paired["comparison"])
        self.assertFalse(self.paired["comparison"]["strict_alignment"])
        self.assertTrue(self.paired["comparison"]["terminal_contract_verified"])
        self.assertEqual(compare_consumers(self.durable,self.agent), self.consumers["comparison"])

    def test_request_mismatch_is_rejected(self) -> None:
        self.opi[0]["request"]["max_completion_tokens"] = 1
        with self.assertRaisesRegex(ValueError,"paired request differs"):
            compare(self.pi,self.opi,self.fixture)

    def test_early_pi_terminal_cannot_be_normalized_away(self) -> None:
        self.pi[0]["checkpoints"]["after_usage"]["eof"] = True
        with self.assertRaisesRegex(ValueError,"Pi ended before sentinel"):
            compare(self.pi,self.opi,self.fixture)

    def test_opi_held_sentinel_change_requires_new_evidence(self) -> None:
        self.opi[2]["checkpoints"]["held_http_eof"]["eof"] = False
        with self.assertRaisesRegex(ValueError,"Opi held EOF timing"):
            compare(self.pi,self.opi,self.fixture)

    def test_hidden_error_outcome_difference_is_rejected(self) -> None:
        self.opi[5]["final"]["result"]["stop"] = "tool_use"
        with self.assertRaisesRegex(ValueError,"Opi final result"):
            compare(self.pi,self.opi,self.fixture)

    def test_missing_pair_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError,"case count differs"):
            compare(self.pi[:-1],self.opi,self.fixture)

    def test_durable_tool_side_effect_is_checked(self) -> None:
        self.durable["observations"][1]["after_usage"]["executed"] = [{"a":"ok","n":2}]
        with self.assertRaisesRegex(ValueError,"early Durable tool"):
            compare_consumers(self.durable,self.agent)

    def test_durable_persistence_is_checked(self) -> None:
        self.durable["observations"][1]["after_usage"]["stored_assistants"] = [{"stop":"tool_use"}]
        with self.assertRaisesRegex(ValueError,"early assistant persistence"):
            compare_consumers(self.durable,self.agent)

    def test_real_agent_side_effect_is_checked(self) -> None:
        self.agent[1]["before_tail"]["tool_calls"] = 1
        with self.assertRaisesRegex(ValueError,"Agent finished before tail"):
            compare_consumers(self.durable,self.agent)

    def test_old_premature_success_evidence_fails_repaired_contract(self) -> None:
        old = json.loads((FIXTURE_DIR / "phase5-chat-finish-timing.json").read_bytes())
        with self.assertRaisesRegex(ValueError, "Opi ended before sentinel"):
            compare(old["pi_runs"][0]["trace"], old["opi_runs"][0]["trace"], self.fixture)

    def test_response_leak_is_rejected(self) -> None:
        self.opi[2]["checkpoints"]["held_http_eof"]["peer_closed"] = False
        with self.assertRaisesRegex(ValueError, "Opi held EOF timing"):
            compare(self.pi, self.opi, self.fixture)

    def test_cancellation_block_end_difference_is_explicit(self) -> None:
        for value in (self.opi[6]["final"], self.opi[6]["checkpoints"]["held_http_eof"]):
            value["events"].insert(-1, {"kind":"toolcall_end"})
        with self.assertRaisesRegex(ValueError, "Opi terminal event sequence"):
            compare(self.pi, self.opi, self.fixture)

    def test_eof_wait_cannot_hide_early_block_ends(self) -> None:
        self.opi[3]["checkpoints"]["held_http_eof"]["events"].append({"kind":"toolcall_end"})
        with self.assertRaisesRegex(ValueError, "blocks ended before natural EOF"):
            compare(self.pi, self.opi, self.fixture)

    def test_held_wrong_result_cannot_be_corrected_by_final_snapshot(self) -> None:
        self.opi[5]["checkpoints"]["held_http_eof"]["result"]["stop"] = "tool_use"
        with self.assertRaisesRegex(ValueError, "held terminal result"):
            compare(self.pi, self.opi, self.fixture)

    def test_cli_existing_receipt_is_rejected_before_loading_inputs(self) -> None:
        for script, flags in (
            ("replay_chat_finish_timing.py", ("materials","rust-build","pi-repo")),
            ("replay_chat_finish_consumers.py", ("paired","materials","agent-build","pi-repo")),
        ):
            with self.subTest(script=script):
                command = [sys.executable,str(Path(__file__).with_name(script))]
                for flag in flags:
                    command.extend(["--"+flag,"missing-input"])
                command.extend(["--output",str(REPO_ROOT / FIXTURE)])
                result = subprocess.run(command,cwd=REPO_ROOT,capture_output=True,text=True,check=False)
                self.assertEqual(result.returncode,2)
                self.assertIn("refusing to overwrite evidence",result.stderr)

    def test_agent_build_input_drift_is_rejected_before_replay(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory) / "build.json"
            build.write_text(json.dumps({"exit_code":0,"inputs":{FIXTURE.as_posix():"0" * 64}}),encoding="utf-8")
            command = [sys.executable,str(Path(__file__).with_name("replay_chat_finish_consumers.py")),
                "--paired",str(FIXTURE_DIR / "phase5-chat-terminal-repaired-verified.json"),
                "--materials","missing-input","--agent-build",str(build),"--pi-repo","missing-input",
                "--output",str(Path(directory) / "unwritten.json")]
            result = subprocess.run(command,cwd=REPO_ROOT,capture_output=True,text=True,check=False)
            self.assertNotEqual(result.returncode,0)
            self.assertIn("Agent build input drift",result.stderr)
            self.assertFalse((Path(directory) / "unwritten.json").exists())


if __name__ == "__main__":
    unittest.main()
