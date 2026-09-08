#!/usr/bin/env python3
"""Deterministic regressions for the offline local-acceptance verifier.

The verifier reads one supplied redacted evidence root and never launches an
Agent, accesses credentials, or contacts a provider. These tests exercise its
structural logic and its Companion invocation contract with a recording stub;
the genuine three-family native acceptance path over the built Companion is
proven by the Rust test in crates/opi-eval/tests/native_driver.rs.
"""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent
VERIFIER = SCRIPTS / "verify-local-acceptance.py"

FAMILIES = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")
PI_REPO = "https://github.com/badlogic/pi-mono"
HARBOR_REPO = "https://github.com/harbor/harbor"
PIER_REPO = "https://github.com/princeton-nlp/SWE-agent"
PI_COMMIT = "a" * 40
HARBOR_COMMIT = "b" * 40
PIER_COMMIT = "c" * 40


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def dir_digest(root: Path) -> str:
    """Recursive LF-stable manifest digest of a directory tree."""
    entries = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root).as_posix()
        if path.is_dir():
            entries.append(f"d {relative}")
        else:
            entries.append(f"f {relative} {sha256(path.read_bytes())}")
    return sha256("\n".join(entries).encode())


def write_exec(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")
    mode = path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
    os.chmod(path, mode)


def write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n"
    )


def companion_stub(record: Path) -> str:
    return (
        "#!/bin/sh\n"
        'set -eu\n'
        f'record="{record}"\n'
        'case "$1 $2" in\n'
        '  "verify --root") [ "$#" -eq 3 ] || { echo "verify argv drift" >&2; exit 9; } ;;\n'
        '  "report --root") [ "$#" -eq 5 ] && [ "$4" = "--out" ] || { echo "report argv drift" >&2; exit 9; } ;;\n'
        '  *) echo "unexpected companion invocation: $*" >&2; exit 9 ;;\n'
        "esac\n"
        '# no credential material may cross the verifier boundary\n'
        'for name in OPENAI_API_KEY ANTHROPIC_API_KEY OPI_EVAL_SCRIPTED_KEY; do\n'
        '  if [ -n "$(eval "echo \\$$name")" ]; then echo "credential env crossed: $name" >&2; exit 9; fi\n'
        "done\n"
        'printf \'%s\\n\' "$*" >> "$record"\n'
        "exit 0\n"
    )


class EvidenceRoot:
    """A synthetic but structurally complete redacted evidence root."""

    def __init__(self, base: Path) -> None:
        self.root = base / "evidence"
        self.root.mkdir(parents=True)
        self.outside = base / "out"
        self.stub_record = base / "companion-invocations.txt"
        self.companion = base / "companion-stub.sh"
        write_exec(self.companion, companion_stub(self.stub_record))
        self.trials = []
        self.runs = {}
        for index, family in enumerate(FAMILIES):
            run = self.root / "runs" / family
            opi_id = f"trial-opi-{index + 1}"
            pi_id = f"trial-pi-{index + 1}"
            self._stage_family(run, family, opi_id, pi_id)
            self.runs[family] = {
                "run": run,
                "opi": opi_id,
                "pi": pi_id,
                "bundle_digests": {},
            }
        self.identity_cache = {
            "schema": "opi-eval-upstream-identity-cache/1",
            "repositories": {
                PI_REPO: {"commits": {PI_COMMIT: {"tree": "1" * 40}}},
                HARBOR_REPO: {"commits": {HARBOR_COMMIT: {"tree": "2" * 40}}},
                PIER_REPO: {"commits": {PIER_COMMIT: {"tree": "3" * 40}}},
            },
        }
        write_json(self.root / "upstream-identity-cache.json", self.identity_cache)
        self._stage_scoped_view()
        self._write_manifest()

    def _stage_family(self, run: Path, family: str, opi_id: str, pi_id: str) -> None:
        write_json(
            run / "run-control.json",
            {
                "schema": "opi-eval-run-control/1",
                "experiment": f"native-acceptance-{family}",
                "planned_trials": [
                    {"id": opi_id, "subject": "candidate-opi", "task": family, "group": "g"},
                    {"id": pi_id, "subject": "baseline-pi", "task": family, "group": "g"},
                ],
            },
        )
        write_json(run / "material.json", {"schema": "preserved-material"})
        for trial, product in ((opi_id, "opi"), (pi_id, "pi")):
            bundle = run / "trials" / trial / "bundle"
            native = bundle / "artifacts" / "native"
            write_json(
                native / "verifier-input.json",
                {"environment_mode": "separate", "collected": []},
            )
            (native / "agent-stdout.log").write_text(
                f"{family} {product} native capture bytes\n", encoding="utf-8"
            )
            write_json(
                bundle / "intent.json",
                {"schema": "opi-eval-intent/1", "product": product},
            )
            write_json(
                bundle / "manifest.json",
                {"schema": "opi-eval-bundle-manifest/1", "artifacts": []},
            )

    def _stage_scoped_view(self) -> None:
        view = self.root / "opi-artifact-audit"
        for family, entry in self.runs.items():
            native = entry["run"] / "trials" / entry["opi"] / "bundle" / "artifacts" / "native"
            target = view / entry["opi"]
            target.mkdir(parents=True)
            for path in sorted(native.iterdir()):
                (target / path.name).write_bytes(path.read_bytes())

    def _write_manifest(self) -> None:
        trials = []
        for family, entry in self.runs.items():
            for key, agent in (("opi", "opi"), ("pi", "pi")):
                trial = entry[key]
                bundle_manifest = (
                    entry["run"] / "trials" / trial / "bundle" / "manifest.json"
                )
                trials.append(
                    {
                        "id": trial,
                        "agent": agent,
                        "family": family,
                        "run_root": f"runs/{family}",
                        "bundle_digest": sha256(bundle_manifest.read_bytes()),
                        "attribution": {"agent_executable_sha256": "e" * 64},
                    }
                )
        manifest = {
            "schema": "opi-eval-local-acceptance/1",
            "operator": {"redacted": True, "label": "local-acceptance-operator"},
            "opi_repository": {"url": "https://github.com/OdradekAI/opi", "commit": "9" * 40},
            "controls": {
                "execution_concurrency": 2,
                "connection_controls": {"selector": "scripted:eval"},
            },
            "trials": trials,
            "sources": {
                "pi": {"repository_url": PI_REPO, "commit": PI_COMMIT},
                "harbor": {"repository_url": HARBOR_REPO, "commit": HARBOR_COMMIT},
                "pier": {"repository_url": PIER_REPO, "commit": PIER_COMMIT},
            },
            "builds": [
                {"agent": "opi", "executable_sha256": "e" * 64},
                {"agent": "pi", "executable_sha256": "e" * 64},
            ],
            "material": [],
            "command_exits": [],
            "preserved": [],
        }
        for family, entry in self.runs.items():
            for command in ("run", "verify", "report"):
                manifest["command_exits"].append(
                    {"family": family, "command": command, "exit": 0}
                )
            manifest["preserved"].append(
                {
                    "role": "material",
                    "path": f"runs/{family}/material.json",
                    "sha256": sha256((entry["run"] / "material.json").read_bytes()),
                }
            )
            manifest["material"].append(
                {
                    "family": family,
                    "material_sha256": sha256(
                        (entry["run"] / "material.json").read_bytes()
                    ),
                }
            )
        write_json(self.root / "manifest.json", manifest)
        self.manifest_path = self.root / "manifest.json"

    def verify(self) -> subprocess.CompletedProcess:
        out = self.outside / f"report-{id(self):x}"
        return subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--root",
                str(self.root),
                "--out",
                str(out),
                "--companion",
                str(self.companion),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )

    def load_manifest(self) -> dict:
        return json.loads(self.manifest_path.read_text(encoding="utf-8"))

    def save_manifest(self, manifest: dict) -> None:
        write_json(self.manifest_path, manifest)


class AcceptanceVerifierTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.base = Path(self._tmp.name)
        self.evidence = EvidenceRoot(self.base)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def accept(self) -> dict:
        result = self.evidence.verify()
        self.assertEqual(
            result.returncode,
            0,
            f"expected acceptance, stderr: {result.stderr}\nstdout: {result.stdout}",
        )
        report = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertTrue(report["accepted"])
        return report

    def reject(self, fragment: str) -> dict:
        result = self.evidence.verify()
        self.assertEqual(
            result.returncode,
            2,
            f"expected rejection, stdout: {result.stdout}\nstderr: {result.stderr}",
        )
        self.assertIn(fragment, result.stderr)
        report = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertFalse(report["accepted"])
        return report

    # -- acceptance ---------------------------------------------------------

    def test_genuine_structured_root_is_accepted(self) -> None:
        report = self.accept()
        self.assertEqual(report["schema"], "opi-eval-local-acceptance-verification/1")
        self.assertEqual(len(report["trials"]), 6)
        out_root = self.evidence.outside
        written = [path.name for path in sorted(out_root.iterdir())]
        self.assertEqual(written, [f"report-{id(self.evidence):x}"])

    def test_companion_invoked_once_per_family_for_verify_and_report(self) -> None:
        self.accept()
        invocations = self.evidence.stub_record.read_text(encoding="utf-8").splitlines()
        self.assertEqual(len(invocations), 6)
        for family in FAMILIES:
            self.assertIn(
                f"verify --root {self.evidence.root / 'runs' / family}", invocations
            )
            reports = [
                line
                for line in invocations
                if line.startswith(f"report --root {self.evidence.root / 'runs' / family}")
            ]
            self.assertEqual(len(reports), 1)
            self.assertIn("--out", reports[0])

    # -- request-level rejections -------------------------------------------

    def test_missing_root_is_rejected(self) -> None:
        self.evidence.root.rename(self.base / "moved")
        result = self.evidence.verify()
        self.assertEqual(result.returncode, 2)
        self.assertIn("evidence root does not exist", result.stderr)

    def test_existing_output_is_rejected(self) -> None:
        out = self.evidence.outside / f"report-{id(self.evidence):x}"
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text("prior", encoding="utf-8")
        result = self.evidence.verify()
        self.assertEqual(result.returncode, 2)
        self.assertIn("output path already exists", result.stderr)

    def test_output_inside_root_is_rejected(self) -> None:
        out = self.evidence.root / "nested-report.json"
        result = subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--root",
                str(self.evidence.root),
                "--out",
                str(out),
                "--companion",
                str(self.evidence.companion),
            ],
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("lies inside the evidence root", result.stderr)

    # -- manifest rejections -------------------------------------------------

    def test_wrong_schema_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["schema"] = "something-else/1"
        self.evidence.save_manifest(manifest)
        self.reject("manifest schema")

    def test_missing_trial_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"] = manifest["trials"][:5]
        self.evidence.save_manifest(manifest)
        self.reject("six attributable trials")

    def test_duplicate_trial_id_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][2]["id"] = manifest["trials"][0]["id"]
        self.evidence.save_manifest(manifest)
        self.reject("six attributable trials")

    def test_oracle_attribution_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][0]["attribution"]["agent_executable_sha256"] = "oracle"
        self.evidence.save_manifest(manifest)
        self.reject("oracle")

    def test_fixture_only_evidence_is_rejected(self) -> None:
        family = FAMILIES[0]
        entry = self.evidence.runs[family]
        for key in ("opi", "pi"):
            native = entry["run"] / "trials" / entry[key] / "bundle" / "artifacts" / "native"
            verdict = native / "verifier-input.json"
            verdict.unlink()
        self.reject("fixture-only")

    def test_substituted_bundle_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["trials"][0]["bundle_digest"] = "0" * 64
        self.evidence.save_manifest(manifest)
        self.reject("bundle digest")

    def test_missing_command_exit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["command_exits"] = manifest["command_exits"][:-1]
        self.evidence.save_manifest(manifest)
        self.reject("command exits")

    def test_nonzero_command_exit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["command_exits"][0]["exit"] = 2
        self.evidence.save_manifest(manifest)
        self.reject("command exits")

    def test_substituted_preserved_artifact_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"][0]["sha256"] = "0" * 64
        self.evidence.save_manifest(manifest)
        self.reject("preserved artifact")

    def test_missing_preserved_artifact_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["preserved"].append(
            {"role": "native", "path": "runs/missing/capture.json", "sha256": sha256(b"x")}
        )
        self.evidence.save_manifest(manifest)
        self.reject("preserved artifact")

    def test_control_drift_against_run_control_is_rejected(self) -> None:
        family = FAMILIES[0]
        control = self.evidence.root / "runs" / family / "run-control.json"
        value = json.loads(control.read_text(encoding="utf-8"))
        value["planned_trials"][0]["subject"] = "someone-else"
        write_json(control, value)
        self.reject("run-control")

    # -- scoped capture view --------------------------------------------------

    def test_empty_scoped_view_is_rejected_even_with_generic_audit_success(self) -> None:
        view = self.evidence.root / "opi-artifact-audit"
        for child in list(view.iterdir()):
            if child.is_dir():
                import shutil

                shutil.rmtree(child)
        write_json(view / "generic-audit-summary.json", {"result": "success"})
        self.reject("empty")

    def test_missing_scoped_view_is_rejected(self) -> None:
        import shutil

        shutil.rmtree(self.evidence.root / "opi-artifact-audit")
        self.reject("opi-artifact-audit")

    def test_substituted_capture_bytes_are_rejected(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        view = self.evidence.root / "opi-artifact-audit" / entry["opi"]
        log = view / "agent-stdout.log"
        log.write_bytes(log.read_bytes() + b"tampered")
        self.reject("byte-bound")

    def test_scoped_view_with_pi_capture_is_not_exhaustive(self) -> None:
        entry = self.evidence.runs[FAMILIES[0]]
        pi_native = entry["run"] / "trials" / entry["pi"] / "bundle" / "artifacts" / "native"
        view = self.evidence.root / "opi-artifact-audit" / entry["pi"]
        view.mkdir()
        for path in sorted(pi_native.iterdir()):
            (view / path.name).write_bytes(path.read_bytes())
        self.reject("exhaustive")

    def test_scoped_view_extra_file_with_non_opi_commit_is_rejected(self) -> None:
        view = self.evidence.root / "opi-artifact-audit"
        (view / "notes.txt").write_text(f"built from {PI_COMMIT}\n", encoding="utf-8")
        self.reject("commit reference")

    # -- upstream identity validation -----------------------------------------

    def test_valid_upstream_identities_are_accepted(self) -> None:
        self.accept()

    def test_mismatched_repository_for_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["pi"]["repository_url"] = HARBOR_REPO
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_unknown_upstream_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["pier"]["commit"] = "d" * 40
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_malformed_upstream_commit_is_rejected(self) -> None:
        manifest = self.evidence.load_manifest()
        manifest["sources"]["harbor"]["commit"] = "not-a-commit"
        self.evidence.save_manifest(manifest)
        self.reject("upstream identity")

    def test_identity_cache_without_repository_is_rejected(self) -> None:
        cache = self.evidence.root / "upstream-identity-cache.json"
        value = json.loads(cache.read_text(encoding="utf-8"))
        del value["repositories"][PIER_REPO]
        write_json(cache, value)
        self.reject("upstream identity")


if __name__ == "__main__":
    unittest.main()
