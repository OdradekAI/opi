#!/usr/bin/env python3
"""Contract tests for the opi-eval materialization artifact verifier.

The verifier audits a manually dispatched materialization run's downloaded
artifact zip and run-produced receipt against the committed static external
lock, the pinned workflow, and the pinned producer bytes, then writes (or
re-verifies) the resolved Linux x86_64 external lock. These tests exercise
the rejection matrix against hermetic temporary workspaces that copy the
real committed repository pins, and the acceptance path against both the
hermetic workspace and the real repository files.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("verify-external-lock-artifact.py")
REPO_ROOT = Path(__file__).resolve().parents[3]

STATIC_LOCK = "crates/opi-eval/external-locks/static/linux-x86_64.json"
WORKFLOW_PATH = ".github/workflows/opi-eval-external-lock-materialization.yml"
MATERIALIZER = "crates/opi-eval/scripts/materialize-external-locks.sh"
CI_VERIFIER = "crates/opi-eval/scripts/verify-external-lock-ci.py"
RESOLVED_LOCK = "crates/opi-eval/external-locks/resolved/linux-x86_64.json"

EXPECTED_COMMIT = "a" * 40
WORKFLOW_SHA = "b" * 40
RUN_ID = 9876543210
RUN_ATTEMPT = 1
ARTIFACT_ID = 456789
ARTIFACT_NAME = "opi-eval-linux-lock-materialization"
ARTIFACT_URL = "https://api.github.com/repos/OdradekAI/opi/actions/artifacts/456789/zip"
RESOLVED_AT = "2030-01-01T00:00:00Z"
EXPIRES_AT = "2030-02-01T00:00:00Z"
CHECK_NOW = "2030-01-15T00:00:00Z"


def lf_sha256(data: bytes) -> str:
    return hashlib.sha256(data.replace(b"\r\n", b"\n")).hexdigest()


def producer_identity(pins: list[dict]) -> str:
    lines = "".join(f"{p['path']}\t{p['sha256']}\n" for p in sorted(pins, key=lambda p: p["path"]))
    return lf_sha256(lines.encode("utf-8"))


class Workspace:
    """A hermetic repository-shaped workspace with a synthetic materialization.

    Copies the real committed static lock, workflow, and producer scripts into
    a temporary root so every pin audit binds to real bytes, then synthesizes
    a structurally valid materialization artifact zip and receipt whose
    digests are bound to those real pins and to the synthesized bytes.
    """

    def __init__(self, root: Path | None = None) -> None:
        self.root = root or Path(tempfile.mkdtemp(prefix="opi-eval-artifact-verify-"))
        for name in (STATIC_LOCK, WORKFLOW_PATH, MATERIALIZER, CI_VERIFIER):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes((REPO_ROOT / name).read_bytes())
        self.static_lock = json.loads((self.root / STATIC_LOCK).read_text(encoding="utf-8"))
        self.static_sha = lf_sha256((self.root / STATIC_LOCK).read_bytes())
        self.producer_pins = [
            {"path": p["path"], "sha256": p["sha256"]}
            for p in self.static_lock["authority"]["producers"]
        ]
        self.closure_files: dict[str, bytes] = {
            "closure/apt/indexes/20250822T180000Z_bookworm_main_binary-amd64_Packages": b"index-1\n",
            "closure/apt/indexes/20250822T180000Z_bookworm-updates_main_binary-amd64_Packages": b"index-2\n",
            "closure/apt/pool/curl_8.9.1-1_amd64.deb": b"curl-deb\n",
            "closure/apt/pool/ca-certificates_20240203_amd64.deb": b"ca-deb\n",
            "closure/uv/uv-installer.sh": b"#!/bin/sh\nuv-installer\n",
            "closure/uv/uv-x86_64-unknown-linux-gnu.tar.gz": b"uv-archive-bytes\n",
            "closure/uv/uv-installer.sha256": b"0" * 64 + b"  uv\n" + b"1" * 64 + b"  uvx\n",
            "closure/wheels/terminal_bench-2.1.0-py3-none-any.whl": b"wheel-one\n",
            "closure/wheels/harbor-0.22.0-py3-none-any.whl": b"wheel-two\n",
            "closure/opt/curl-shim.sh": b"#!/usr/bin/env bash\nexec /usr/bin/curl \"$@\"\n",
            "closure/opt/apt-closure.conf": b"APT_CONF_CLOSED=1\n",
        }
        self.oracle_files: dict[str, bytes] = {
            "oracle/harbor-results.json": b'[{"task_id": "openssl-selfsigned-cert", "reward": 1.0}]\n',
            "oracle/oracle-summary.json": b'{"rewards": [1.0]}\n',
            "oracle/results.ctrf": b'{"results": {"tests": [{"state": "passed"}]}}\n',
            "oracle/network-baseline.json": b"{}\n",
            "oracle/network-inspection.txt": (
                b"network-mode: none\n"
                b"probe: docker run --network none curl https://example.com/ -> refused\n"
                b"baseline: " + lf_sha256(b"{}\n").encode("ascii") + b"\n"
            ),
        }
        self.pinned_image = self.static_lock["images"][0]
        self._installed: dict[str, Path] | None = None
        self.pulled_images_bytes = (
            json.dumps(
                {
                    "id": self.pinned_image["id"],
                    "reference": self.pinned_image["reference"],
                    "manifest": self.pinned_image["manifest"],
                    "config": self.pinned_image["config"],
                    "layers": self.pinned_image["layers"],
                },
                sort_keys=True,
            )
            + "\n"
        ).encode("utf-8")

    # -- artifact assembly ---------------------------------------------------

    def closure_manifest(self) -> tuple[str, int, int]:
        records = []
        for name, body in self.closure_files.items():
            rel = name[len("closure/") :]
            role = (
                "apt-index"
                if rel.startswith("apt/indexes/")
                else "apt-package"
                if rel.startswith("apt/pool/")
                else "uv-asset"
                if rel.startswith("uv/")
                else "wheel"
                if rel.startswith("wheels/")
                else "adapter-policy"
            )
            records.append(
                {
                    "path": rel,
                    "role": role,
                    "size": len(body),
                    "sha256": hashlib.sha256(body).hexdigest(),
                }
            )
        records.sort(key=lambda r: r["path"])
        manifest = hashlib.sha256(
            "".join(f"{r['path']}\t{r['role']}\t{r['size']}\t{r['sha256']}\n" for r in records).encode()
        ).hexdigest()
        return manifest, len(records), sum(r["size"] for r in records)

    def build_receipt(self, **overrides: object) -> dict:
        manifest, count, total = self.closure_manifest()
        receipt = {
            "schema": "opi-eval-materialization-receipt/1",
            "lock_id": "opi-eval-linux-x86_64",
            "platform": "linux-x86_64",
            "candidate_commit": EXPECTED_COMMIT,
            "static_lock": {"path": STATIC_LOCK, "sha256": self.static_sha},
            "workflow": {
                "ref": "refs/heads/main",
                "sha": WORKFLOW_SHA,
                "path": WORKFLOW_PATH,
                "bytes_sha256": self.static_lock["authority"]["workflow"]["sha256"],
            },
            "producers": [dict(p) for p in self.producer_pins],
            "run": {"id": RUN_ID, "attempt": RUN_ATTEMPT},
            "resolved_at": RESOLVED_AT,
            "artifact_name": ARTIFACT_NAME,
            "expires_at": EXPIRES_AT,
            "closure": {
                "manifest_sha256": manifest,
                "file_count": count,
                "total_bytes": total,
            },
            "images": [json.loads(self.pulled_images_bytes.decode("utf-8"))],
            "oracle": {
                "status": "passed",
                "reward": 1.0,
                "ctrf_sha256": hashlib.sha256(self.oracle_files["oracle/results.ctrf"]).hexdigest(),
                "harbor_results_sha256": lf_sha256(self.oracle_files["oracle/harbor-results.json"]),
            },
            "network": {"mode": "none", "evidence": "oracle/network-inspection.txt"},
        }
        receipt.update(overrides)
        return receipt

    def build_zip(
        self,
        receipt: dict | None = None,
        receipt_bytes: bytes | None = None,
        extra_members: dict[str, bytes] | None = None,
        omit_members: list[str] | None = None,
    ) -> bytes:
        receipt_bytes = receipt_bytes or (
            json.dumps(self.build_receipt() if receipt is None else receipt, sort_keys=True, indent=2)
            + "\n"
        ).encode("utf-8")
        manifest, _, _ = self.closure_manifest()
        closure_manifest_bytes = (
            json.dumps(
                {
                    "manifest_sha256": manifest,
                    "files": [
                        {
                            "path": name[len("closure/") :],
                            "role": (
                                "apt-index"
                                if name.startswith("closure/apt/indexes/")
                                else "apt-package"
                                if name.startswith("closure/apt/pool/")
                                else "uv-asset"
                                if name.startswith("closure/uv/")
                                else "wheel"
                                if name.startswith("closure/wheels/")
                                else "adapter-policy"
                            ),
                            "size": len(body),
                            "sha256": hashlib.sha256(body).hexdigest(),
                        }
                        for name, body in sorted(self.closure_files.items())
                    ],
                },
                sort_keys=True,
                indent=2,
            )
            + "\n"
        ).encode("utf-8")
        members: dict[str, bytes] = {
            **self.closure_files,
            **self.oracle_files,
            "receipt.json": receipt_bytes,
            "closure-manifest.json": closure_manifest_bytes,
            "pulled-images.json": self.pulled_images_bytes,
            "task-image-manifest.json": b'{"schemaVersion": 2, "config": {}, "layers": []}\n',
        }
        for name in omit_members or []:
            members.pop(name, None)
        members.update(extra_members or {})
        import io

        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED) as archive:
            for name, body in members.items():
                archive.writestr(name, body)
        return buffer.getvalue()

    # -- invocation ----------------------------------------------------------

    def install(self, receipt: dict | None = None, zip_bytes: bytes | None = None) -> dict[str, Path]:
        if self._installed is not None and receipt is None and zip_bytes is None:
            return self._installed
        receipt_bytes = (
            json.dumps(self.build_receipt() if receipt is None else receipt, sort_keys=True, indent=2)
            + "\n"
        ).encode("utf-8")
        zip_bytes = zip_bytes or self.build_zip(receipt_bytes=receipt_bytes)
        receipt_path = self.root / "downloaded-receipt.json"
        receipt_path.write_bytes(receipt_bytes)
        artifact_path = self.root / "downloaded-artifact.zip"
        artifact_path.write_bytes(zip_bytes)
        self._installed = {"receipt": receipt_path, "artifact": artifact_path}
        return self._installed

    def run(
        self,
        *extra_args: str,
        receipt_path: Path | None = None,
        artifact_path: Path | None = None,
        artifact_digest: str | None = None,
        expected_commit: str = EXPECTED_COMMIT,
        check_now: str = CHECK_NOW,
        lock_mode: tuple[str, Path] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        if receipt_path is None or artifact_path is None:
            paths = self.install()
            receipt_path = receipt_path or paths["receipt"]
            artifact_path = artifact_path or paths["artifact"]
        digest = artifact_digest or f"sha256:{lf_sha256(artifact_path.read_bytes())}"
        mode, lock_path = lock_mode or ("--write-lock", self.root / RESOLVED_LOCK)
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(self.root),
                "--expected-commit",
                expected_commit,
                "--receipt",
                str(receipt_path),
                "--artifact",
                str(artifact_path),
                "--artifact-id",
                str(ARTIFACT_ID),
                "--artifact-url",
                ARTIFACT_URL,
                "--artifact-digest",
                digest,
                "--now",
                check_now,
                *extra_args,
                mode,
                str(lock_path),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )

    def run_with_cross_checks(
        self,
        workflow_sha: str | None = None,
        workflow_ref: str | None = None,
        run_id: int | None = None,
    ) -> subprocess.CompletedProcess[str]:
        extra: list[str] = []
        if workflow_sha is not None:
            extra += ["--workflow-sha", workflow_sha]
        if workflow_ref is not None:
            extra += ["--workflow-ref", workflow_ref]
        if run_id is not None:
            extra += ["--run-id", str(run_id)]
        return self.run(*extra)

    # -- mutations -----------------------------------------------------------

    def rerun_with_receipt(self, receipt: dict, **run_kwargs: object) -> subprocess.CompletedProcess[str]:
        receipt_bytes = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode("utf-8")
        zip_bytes = self.build_zip(receipt_bytes=receipt_bytes)
        paths = self.install(receipt=None, zip_bytes=zip_bytes)
        paths["receipt"].write_bytes(receipt_bytes)
        return self.run(
            receipt_path=paths["receipt"],
            artifact_path=paths["artifact"],
            **run_kwargs,  # type: ignore[arg-type]
        )


def output(result: subprocess.CompletedProcess[str]) -> str:
    return result.stdout + result.stderr


class RejectsMissingContext(unittest.TestCase):
    def base_argv(self) -> list[str]:
        paths = Workspace().install()
        return [
            sys.executable,
            str(SCRIPT),
            "--root",
            str(REPO_ROOT),
            "--expected-commit",
            EXPECTED_COMMIT,
            "--receipt",
            str(paths["receipt"]),
            "--artifact",
            str(paths["artifact"]),
            "--artifact-id",
            str(ARTIFACT_ID),
            "--artifact-url",
            ARTIFACT_URL,
            "--artifact-digest",
            f"sha256:{lf_sha256(paths['artifact'].read_bytes())}",
        ]

    def test_missing_required_argument_rejects(self) -> None:
        for dropped in ("--receipt", "--artifact", "--expected-commit", "--artifact-digest"):
            argv = self.base_argv()
            index = argv.index(dropped)
            del argv[index : index + 2]
            result = subprocess.run(argv, capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(result.returncode, 2, dropped)

    def test_lock_mode_is_required_exactly_once(self) -> None:
        argv = self.base_argv()
        result = subprocess.run(argv, capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 2, output(result))
        argv = self.base_argv() + ["--write-lock", "/tmp/x", "--verify-lock", "/tmp/x"]
        result = subprocess.run(argv, capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 2, output(result))


class RejectsReceiptContract(unittest.TestCase):
    def test_invalid_json_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        paths["receipt"].write_bytes(b"{not json")
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("receipt", output(result).lower())

    def test_wrong_schema_or_identity_rejects(self) -> None:
        for field, value in (
            ("schema", "opi-eval-materialization-receipt/2"),
            ("lock_id", "opi-eval-other"),
            ("platform", "linux-aarch64"),
        ):
            with self.subTest(field=field):
                ws = Workspace()
                result = ws.rerun_with_receipt(ws.build_receipt(**{field: value}))
                self.assertEqual(result.returncode, 2, output(result))
                self.assertIn(field, output(result).lower())

    def test_candidate_commit_mismatch_rejects(self) -> None:
        ws = Workspace()
        result = ws.rerun_with_receipt(
            ws.build_receipt(candidate_commit="f" * 40), expected_commit="e" * 40
        )
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("candidate", output(result).lower())

    def test_malformed_candidate_commit_rejects(self) -> None:
        ws = Workspace()
        result = ws.rerun_with_receipt(ws.build_receipt(candidate_commit="short"))
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("commit", output(result).lower())


class RejectsStaticBinding(unittest.TestCase):
    def test_receipt_static_digest_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["static_lock"]["sha256"] = "0" * 64
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("static", output(result).lower())

    def test_receipt_static_path_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["static_lock"]["path"] = "elsewhere/static.json"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("static", output(result).lower())

    def test_local_static_lock_drift_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        lock_path = ws.root / STATIC_LOCK
        lock_path.write_bytes(lock_path.read_bytes() + b"\n")
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("static", output(result).lower())


class RejectsLocalProducerDrift(unittest.TestCase):
    def test_producer_bytes_drift_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        (ws.root / MATERIALIZER).write_bytes(b"#!/usr/bin/env bash\n# tampered\n")
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("producer", output(result).lower())


class RejectsWorkflowBinding(unittest.TestCase):
    def test_receipt_workflow_path_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["workflow"]["path"] = ".github/workflows/other.yml"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_receipt_workflow_bytes_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["workflow"]["bytes_sha256"] = "2" * 64
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_local_workflow_drift_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        workflow = ws.root / WORKFLOW_PATH
        workflow.write_bytes(workflow.read_bytes() + b"# drifted\n")
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_malformed_workflow_identity_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["workflow"]["sha"] = "not-hex"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        receipt = ws.build_receipt()
        receipt["workflow"]["ref"] = "unknown"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_qualified_github_workflow_ref_normalizes(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["workflow"]["ref"] = (
            "OdradekAI/opi/.github/workflows/opi-eval-external-lock-materialization.yml@refs/heads/main"
        )
        lock_path = ws.root / RESOLVED_LOCK
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 0, output(result))
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        self.assertEqual(lock["workflow"]["ref"], "refs/heads/main")

    def test_workflow_ref_without_refs_component_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["workflow"]["ref"] = "OdradekAI/opi/some/workflow.yml@main"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_cross_check_sha_mismatch_rejects(self) -> None:
        ws = Workspace()
        result = ws.run_with_cross_checks(workflow_sha="c" * 40)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_cross_check_ref_mismatch_rejects(self) -> None:
        ws = Workspace()
        result = ws.run_with_cross_checks(workflow_ref="refs/heads/other")
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("workflow", output(result).lower())

    def test_cross_check_run_id_mismatch_rejects(self) -> None:
        ws = Workspace()
        result = ws.run_with_cross_checks(run_id=RUN_ID + 1)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("run", output(result).lower())


class RejectsUnboundRun(unittest.TestCase):
    def test_zero_run_id_rejects(self) -> None:
        ws = Workspace()
        result = ws.rerun_with_receipt(ws.build_receipt(run={"id": 0, "attempt": 1}))
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("run", output(result).lower())

    def test_zero_attempt_rejects(self) -> None:
        ws = Workspace()
        result = ws.rerun_with_receipt(ws.build_receipt(run={"id": RUN_ID, "attempt": 0}))
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("run", output(result).lower())


class RejectsArtifactBinding(unittest.TestCase):
    def test_digest_mismatch_rejects(self) -> None:
        ws = Workspace()
        result = ws.run(artifact_digest=f"sha256:{'3' * 64}")
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("artifact", output(result).lower())

    def test_malformed_digest_rejects(self) -> None:
        ws = Workspace()
        result = ws.run(artifact_digest="3" * 64)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("artifact", output(result).lower())

    def test_http_url_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        digest = f"sha256:{lf_sha256(paths['artifact'].read_bytes())}"
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--expected-commit",
                EXPECTED_COMMIT,
                "--receipt",
                str(paths["receipt"]),
                "--artifact",
                str(paths["artifact"]),
                "--artifact-id",
                str(ARTIFACT_ID),
                "--artifact-url",
                "http://example.com/artifact.zip",
                "--artifact-digest",
                digest,
                "--now",
                CHECK_NOW,
                "--write-lock",
                str(ws.root / RESOLVED_LOCK),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("url", output(result).lower())

    def test_zero_artifact_id_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        digest = f"sha256:{lf_sha256(paths['artifact'].read_bytes())}"
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(ws.root),
                "--expected-commit",
                EXPECTED_COMMIT,
                "--receipt",
                str(paths["receipt"]),
                "--artifact",
                str(paths["artifact"]),
                "--artifact-id",
                "0",
                "--artifact-url",
                ARTIFACT_URL,
                "--artifact-digest",
                digest,
                "--now",
                CHECK_NOW,
                "--write-lock",
                str(ws.root / RESOLVED_LOCK),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("artifact", output(result).lower())

    def test_non_zip_artifact_rejects(self) -> None:
        ws = Workspace()
        paths = ws.install()
        paths["artifact"].write_bytes(b"not a zip archive")
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("artifact", output(result).lower())

    def test_zip_without_receipt_member_rejects(self) -> None:
        ws = Workspace()
        zip_bytes = ws.build_zip(omit_members=["receipt.json"])
        paths = ws.install(zip_bytes=zip_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("receipt", output(result).lower())

    def test_zip_receipt_diverges_from_standalone_receipt(self) -> None:
        ws = Workspace()
        zip_bytes = ws.build_zip(
            receipt_bytes=(json.dumps(ws.build_receipt(), sort_keys=True, indent=2) + "\n").encode()
        )
        diverged = ws.build_receipt()
        diverged["run"] = {"id": RUN_ID + 5, "attempt": 1}
        receipt_bytes = (json.dumps(diverged, sort_keys=True, indent=2) + "\n").encode()
        paths = ws.install(zip_bytes=zip_bytes)
        paths["receipt"].write_bytes(receipt_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("receipt", output(result).lower())


class RejectsClosureAudit(unittest.TestCase):
    def audit_with(self, mutate_zip: dict[str, bytes] | None = None, receipt=None) -> str:
        ws = Workspace()
        zip_bytes = ws.build_zip(extra_members=mutate_zip)
        receipt = receipt if receipt is not None else ws.build_receipt()
        receipt_bytes = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode()
        zip_bytes = ws.build_zip(receipt_bytes=receipt_bytes, extra_members=mutate_zip)
        paths = ws.install(zip_bytes=zip_bytes)
        paths["receipt"].write_bytes(receipt_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        return output(result)

    def test_extra_closure_member_rejects(self) -> None:
        text = self.audit_with(mutate_zip={"closure/apt/pool/rogue.deb": b"rogue\n"})
        self.assertIn("closure", text.lower())

    def test_modified_closure_member_rejects(self) -> None:
        text = self.audit_with(mutate_zip={"closure/opt/apt-closure.conf": b"tampered\n"})
        self.assertIn("closure", text.lower())

    def test_receipt_closure_digest_mismatch_rejects(self) -> None:
        receipt = Workspace().build_receipt()
        receipt["closure"]["manifest_sha256"] = "4" * 64
        text = self.audit_with(receipt=receipt)
        self.assertIn("closure", text.lower())

    def test_receipt_closure_count_mismatch_rejects(self) -> None:
        receipt = Workspace().build_receipt()
        receipt["closure"]["file_count"] = 1
        text = self.audit_with(receipt=receipt)
        self.assertIn("closure", text.lower())

    def test_missing_closure_directory_rejects(self) -> None:
        ws = Workspace()
        zip_bytes = ws.build_zip(
            omit_members=[name for name in ws.closure_files if name.startswith("closure/")]
        )
        paths = ws.install(zip_bytes=zip_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("closure", output(result).lower())


class RejectsImageAudit(unittest.TestCase):
    def test_diverged_pulled_images_rejects(self) -> None:
        ws = Workspace()
        diverged = json.loads(ws.pulled_images_bytes.decode("utf-8"))
        diverged["config"] = f"sha256:{'5' * 64}"
        zip_bytes = ws.build_zip(
            extra_members={"pulled-images.json": (json.dumps(diverged, sort_keys=True) + "\n").encode()}
        )
        paths = ws.install(zip_bytes=zip_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("image", output(result).lower())

    def test_receipt_image_manifest_off_pin_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["images"][0]["manifest"] = f"sha256:{'6' * 64}"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("image", output(result).lower())

    def test_empty_layers_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["images"][0]["layers"] = []
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("layer", output(result).lower())

    def test_malformed_config_digest_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["images"][0]["config"] = "deadbeef"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("config", output(result).lower())


class RejectsOracleAudit(unittest.TestCase):
    def test_failed_status_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["oracle"]["status"] = "failed"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("oracle", output(result).lower())

    def test_zero_reward_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["oracle"]["reward"] = 0.0
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("reward", output(result).lower())

    def test_ctrf_digest_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["oracle"]["ctrf_sha256"] = "7" * 64
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("ctrf", output(result).lower())

    def test_missing_ctrf_member_rejects(self) -> None:
        ws = Workspace()
        zip_bytes = ws.build_zip(omit_members=["oracle/results.ctrf"])
        paths = ws.install(zip_bytes=zip_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("ctrf", output(result).lower())

    def test_harbor_results_digest_mismatch_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["oracle"]["harbor_results_sha256"] = "8" * 64
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("harbor", output(result).lower())

    def test_missing_network_evidence_rejects(self) -> None:
        ws = Workspace()
        zip_bytes = ws.build_zip(omit_members=["oracle/network-inspection.txt"])
        paths = ws.install(zip_bytes=zip_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("network", output(result).lower())

    def test_unknown_network_mode_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["network"]["mode"] = "bridge"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("network", output(result).lower())

    def test_network_mode_diverging_from_evidence_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["network"]["mode"] = "public"
        receipt_bytes = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode("utf-8")
        zip_bytes = ws.build_zip(receipt_bytes=receipt_bytes)
        paths = ws.install(zip_bytes=zip_bytes)
        paths["receipt"].write_bytes(receipt_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("network", output(result).lower())

    def test_public_network_mode_consistent_with_evidence_accepts(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["network"]["mode"] = "public"
        receipt_bytes = (json.dumps(receipt, sort_keys=True, indent=2) + "\n").encode("utf-8")
        evidence = ws.oracle_files["oracle/network-inspection.txt"].replace(
            b"network-mode: none", b"network-mode: public"
        )
        zip_bytes = ws.build_zip(
            receipt_bytes=receipt_bytes,
            extra_members={"oracle/network-inspection.txt": evidence},
        )
        paths = ws.install(zip_bytes=zip_bytes)
        paths["receipt"].write_bytes(receipt_bytes)
        result = ws.run(receipt_path=paths["receipt"], artifact_path=paths["artifact"])
        self.assertEqual(result.returncode, 0, output(result))


class RejectsExpiry(unittest.TestCase):
    def test_expired_artifact_rejects(self) -> None:
        ws = Workspace()
        result = ws.run(check_now="2030-03-01T00:00:00Z")
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("expire", output(result).lower())

    def test_resolved_not_before_expiry_rejects(self) -> None:
        ws = Workspace()
        receipt = ws.build_receipt()
        receipt["resolved_at"] = "2030-02-01T00:00:00Z"
        result = ws.rerun_with_receipt(receipt)
        self.assertEqual(result.returncode, 2, output(result))
        self.assertIn("timestamp", output(result).lower())

    def test_malformed_timestamps_reject(self) -> None:
        for field, value in (
            ("resolved_at", "2030-01-01 00:00:00Z"),
            ("expires_at", "2030-13-01T00:00:00Z"),
        ):
            with self.subTest(field=field):
                ws = Workspace()
                receipt = ws.build_receipt(**{field: value})
                result = ws.rerun_with_receipt(receipt)
                self.assertEqual(result.returncode, 2, output(result))
                self.assertIn("timestamp", output(result).lower())


class AcceptsAndWritesLock(unittest.TestCase):
    def test_seeded_workspace_writes_expected_lock(self) -> None:
        ws = Workspace()
        lock_path = ws.root / RESOLVED_LOCK
        result = ws.run(lock_mode=("--write-lock", lock_path))
        self.assertEqual(result.returncode, 0, output(result))
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        self.assertEqual(lock["schema"], "opi-eval-external-lock/resolved/1")
        self.assertEqual(lock["lock_id"], "opi-eval-linux-x86_64")
        self.assertEqual(lock["platform"], "linux-x86_64")
        self.assertEqual(lock["resolved_by_stage"], "external-lock-materialization")
        self.assertEqual(lock["static_lock"], {"path": STATIC_LOCK, "sha256": ws.static_sha})
        self.assertEqual(
            lock["workflow"],
            {
                "ref": "refs/heads/main",
                "sha": WORKFLOW_SHA,
                "path": WORKFLOW_PATH,
                "bytes_sha256": ws.static_lock["authority"]["workflow"]["sha256"],
            },
        )
        self.assertEqual(
            sorted((p["path"], p["sha256"]) for p in lock["producers"]),
            sorted((p["path"], p["sha256"]) for p in ws.producer_pins),
        )
        self.assertEqual(lock["run"], {"id": RUN_ID, "attempt": RUN_ATTEMPT})
        self.assertEqual(
            lock["artifact"],
            {
                "name": ARTIFACT_NAME,
                "id": ARTIFACT_ID,
                "digest": f"sha256:{lf_sha256((ws.root / 'downloaded-artifact.zip').read_bytes())}",
                "url": ARTIFACT_URL,
                "expires_at": EXPIRES_AT,
            },
        )
        manifest, count, _ = ws.closure_manifest()
        self.assertEqual(lock["closure"], {"manifest_sha256": manifest, "file_count": count})
        self.assertEqual(
            lock["images"],
            [
                {
                    "id": ws.pinned_image["id"],
                    "manifest": ws.pinned_image["manifest"],
                    "config": ws.pinned_image["config"],
                    "layers": ws.pinned_image["layers"],
                }
            ],
        )
        receipt = ws.build_receipt()
        self.assertEqual(
            lock["oracle"],
            {
                "status": "passed",
                "reward": 1.0,
                "ctrf_sha256": receipt["oracle"]["ctrf_sha256"],
            },
        )
        self.assertEqual(lock["authority"], {"admission": "digest"})
        self.assertEqual(
            {slot["id"]: slot["identity"] for slot in lock["resolved"]},
            {
                "materializer-tools": producer_identity(ws.producer_pins),
                "closure-artifact": manifest,
                "pulled-image-graphs": lf_sha256(ws.pulled_images_bytes),
                "harbor-effective-config": receipt["oracle"]["harbor_results_sha256"],
                "docker-network-evidence": lf_sha256(ws.oracle_files["oracle/network-inspection.txt"]),
                "oracle-preflight": receipt["oracle"]["ctrf_sha256"],
            },
        )
        declared = ws.static_lock["unresolved"]
        future_ids = {slot["id"]: slot["owner_stage"] for slot in lock["future"]}
        self.assertEqual(
            future_ids,
            {slot["id"]: slot["owner_stage"] for slot in declared if slot["owner_stage"] != "external-lock-materialization"},
        )
        self.assertEqual(
            {slot["id"] for slot in lock["resolved"]},
            {slot["id"] for slot in declared if slot["owner_stage"] == "external-lock-materialization"},
        )

    def test_write_is_idempotent_and_verify_lock_reaccepts(self) -> None:
        ws = Workspace()
        lock_path = ws.root / RESOLVED_LOCK
        first = ws.run(lock_mode=("--write-lock", lock_path))
        self.assertEqual(first.returncode, 0, output(first))
        written = lock_path.read_bytes()
        second = ws.run(lock_mode=("--write-lock", lock_path))
        self.assertEqual(second.returncode, 0, output(second))
        self.assertEqual(lock_path.read_bytes(), written)
        verify = ws.run(lock_mode=("--verify-lock", lock_path))
        self.assertEqual(verify.returncode, 0, output(verify))

    def test_verify_lock_rejects_drifted_lock(self) -> None:
        ws = Workspace()
        lock_path = ws.root / RESOLVED_LOCK
        result = ws.run(lock_mode=("--write-lock", lock_path))
        self.assertEqual(result.returncode, 0, output(result))
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        lock["closure"]["file_count"] = 1
        lock_path.write_text(json.dumps(lock, sort_keys=True, indent=2) + "\n", encoding="utf-8")
        verify = ws.run(lock_mode=("--verify-lock", lock_path))
        self.assertEqual(verify.returncode, 2, output(verify))
        self.assertIn("lock", output(verify).lower())


class AcceptsRealRepositoryPins(unittest.TestCase):
    def test_real_repository_static_pins_audit(self) -> None:
        ws = Workspace(root=Path(tempfile.mkdtemp(prefix="opi-eval-real-root-")))
        # The hermetic workspace already copied the real pins; run against the
        # real repository root instead, keeping only the synthetic download.
        paths = ws.install()
        digest = f"sha256:{lf_sha256(paths['artifact'].read_bytes())}"
        result = subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                "--root",
                str(REPO_ROOT),
                "--expected-commit",
                EXPECTED_COMMIT,
                "--receipt",
                str(paths["receipt"]),
                "--artifact",
                str(paths["artifact"]),
                "--artifact-id",
                str(ARTIFACT_ID),
                "--artifact-url",
                ARTIFACT_URL,
                "--artifact-digest",
                digest,
                "--now",
                CHECK_NOW,
                "--write-lock",
                str(ws.root / RESOLVED_LOCK),
            ],
            capture_output=True,
            text=True,
            encoding="utf-8",
        )
        self.assertEqual(result.returncode, 0, output(result))


if __name__ == "__main__":
    unittest.main()
