#!/usr/bin/env python3
"""Hermetic tests for crates/opi-eval/scripts/derive-phase18-seam-matrix.py.

The derivation is tested against synthetic sealed stages small enough to
build in a temp directory: the field/evidence classification, the
lifecycle-edge rejections, and the fail-closed chain behavior (a stage
without a sealed manifest never reaches derivation).

Usage:
    python crates/opi-eval/scripts/test_derive_phase18_seam_matrix.py
"""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent


def load(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / filename)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


derive = load("derive_phase18_seam_matrix", "derive-phase18-seam-matrix.py")
native = load("verify_phase18_native_artifact",
              "verify-phase18-native-artifact.py")

REVISIONS = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
AGGREGATE_FOR = {
    "terminal-bench-2.1": "harbor-result",
    "terminal-bench-3.0": "harbor-result",
    "deepswe-v1.1": "pier-result",
}


def write_trial(root: Path, revision: str, product: str, *,
                with_trace: bool = True,
                pre_seal: str | None = "a" * 64,
                seal_binds: bool = True,
                aggregate: bool = True,
                opi_evidence: bool = True,
                pi_evidence: bool = True) -> Path:
    trial_id = f"trial-{product}-{revision}"
    trial = root / "07-trials" / revision / "trials" / trial_id
    (trial / "bundle" / "artifacts" / "native").mkdir(parents=True)
    bundle_identity = "b" * 64
    receipt = {
        "id": trial_id,
        "agent": {
            "product": product,
            "completion": "completed",
            "exit_state": "exited:0",
            "stdout_bytes": 10,
        },
        "verifier": {
            "completion": "verified",
            "exit_state": "exited:0",
            "reward": "known:1(test)",
        },
        "bundle_identity": bundle_identity,
    }
    if pre_seal is not None:
        receipt["pre_seal_digest"] = pre_seal
        receipt["seal_result"] = {"sealed": {"bundle_digest": (
            bundle_identity if seal_binds else "c" * 64)}}
    (trial / "receipt.json").write_text(
        json.dumps(receipt), encoding="utf-8")
    (trial / "bundle" / "settlement.json").write_text("{}", encoding="utf-8")
    artifacts = trial / "bundle" / "artifacts" / "native"
    (artifacts / "agent-stdout.log").write_text("out", encoding="utf-8")
    (artifacts / "verifier-stdout.log").write_text("vout", encoding="utf-8")
    if aggregate:
        (artifacts / "native").mkdir()
        (artifacts / "native" / AGGREGATE_FOR[revision]).write_text(
            "{}", encoding="utf-8")
    if product == "opi" and opi_evidence:
        (artifacts / "evidence").mkdir()
        (artifacts / "evidence" / "manifest").write_text("m", encoding="utf-8")
        (artifacts / "evidence" / "records").write_text("r", encoding="utf-8")
    if product == "pi" and pi_evidence:
        (artifacts / "agent-answer.txt").write_text("ans", encoding="utf-8")
        (artifacts / "events").mkdir()
        (artifacts / "events" / "stdout").write_text("ev", encoding="utf-8")
    if with_trace:
        (trial / "agent-trace").mkdir()
        (trial / "agent-trace" / "events.jsonl").write_text(
            "{}", encoding="utf-8")
        (trial / "verifier-trace").mkdir()
        (trial / "workspace").mkdir()
    return trial


def build_stage(root: Path, **overrides) -> Path:
    for revision in REVISIONS:
        for product in ("opi", "pi"):
            write_trial(root, revision, product, **overrides)
        (root / "07-trials" / revision / "run-report.json").write_text(
            json.dumps({"schema": "phase18-run-report/1", "trials": [
                {"id": f"trial-opi-{revision}",
                 "agent": {"product": "opi"}},
                {"id": f"trial-pi-{revision}",
                 "agent": {"product": "pi"}},
            ]}), encoding="utf-8")
    return root


def reports_for(root: Path) -> list[dict]:
    f = native.Findings()
    return native.load_run_reports(root, f)


class MatrixDerivation(unittest.TestCase):
    def test_classification_from_exact_records(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp))
            matrix = derive.derive_matrix(root, reports_for(root))
            self.assertEqual(matrix["trials"], 6)
            self.assertIn("agent.completion", matrix["shared_fields"])
            self.assertIn("verifier.reward", matrix["shared_fields"])
            self.assertEqual(
                matrix["private_evidence"]["opi"],
                ["native/evidence/manifest", "native/evidence/records"],
            )
            self.assertIn("native/agent-answer.txt",
                          matrix["private_evidence"]["pi"])
            self.assertIn("native/events/stdout",
                          matrix["private_evidence"]["pi"])
            self.assertEqual(
                matrix["rejected_evidence"],
                ["native/native/harbor-result", "native/native/pier-result"],
            )
            for field in matrix["shared_evidence"]:
                self.assertNotIn("native/native", field)

    def test_rendering_is_deterministic(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp))
            matrix = derive.derive_matrix(root, reports_for(root))
            binding = {
                "candidate_commit": "d" * 40,
                "run_id": 1,
                "artifact_digest": "e" * 64,
                "sealed_manifest_sha256": "f" * 64,
                "trials": matrix["trials"],
            }
            first = derive.render_matrix(matrix, binding)
            second = derive.render_matrix(matrix, binding)
            self.assertEqual(first, second)
            self.assertIn("phase18-seam-evidence-matrix/1", first)
            self.assertIn("| verifier-forked | both |", first)


class LifecycleEdges(unittest.TestCase):
    def test_complete_stage_records_every_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp))
            f = native.Findings()
            derive.require_lifecycle_edges(root, reports_for(root), f)
            self.assertEqual([], f.errors)

    def test_missing_agent_trace_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp), with_trace=False)
            f = native.Findings()
            derive.require_lifecycle_edges(root, reports_for(root), f)
            self.assertTrue(
                any("agent-activity" in row for row in f.errors), f.errors)

    def test_missing_pre_seal_digest_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp), pre_seal=None)
            f = native.Findings()
            derive.require_lifecycle_edges(root, reports_for(root), f)
            self.assertTrue(
                any("pre-seal digest is missing" in row for row in f.errors),
                f.errors,
            )

    def test_unbound_seal_result_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp), seal_binds=False)
            f = native.Findings()
            derive.require_lifecycle_edges(root, reports_for(root), f)
            self.assertTrue(
                any("does not bind the typed trajectory" in row
                    for row in f.errors),
                f.errors,
            )


class ChainFailClosed(unittest.TestCase):
    def test_stage_without_sealed_manifest_never_derives(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = build_stage(Path(tmp))
            receipt = Path(tmp) / "receipt.json"
            receipt.write_text(json.dumps({
                "schema": native.UPLOAD_SCHEMA,
                "status": "ok",
                "artifact_digest": "0" * 64,
                "sealed_manifest_sha256": "1" * 64,
                "expires_at": "2999-01-01T00:00:00Z",
                "outer_receipt_sha256": "2" * 64,
            }), encoding="utf-8")
            code = derive.main([
                "--receipt", str(receipt),
                "--artifact", str(root),
                "--require-trajectory-spans",
                "--output", str(Path(tmp) / "matrix.md"),
                "--repo", str(Path(tmp)),
            ])
            self.assertEqual(code, 1)
            self.assertFalse((Path(tmp) / "matrix.md").exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
