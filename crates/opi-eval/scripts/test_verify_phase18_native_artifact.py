#!/usr/bin/env python3
"""Behavioral tests for crates/opi-eval/scripts/verify-phase18-native-artifact.py.

The verifier is the sole owner of the downloaded native-smoke evidence:
it consumes the upload-identity receipt plus the downloaded artifact
(GitHub zip or bare sealed tar) and re-derives every binding the task
18.15 definition of done requires. These tests build a synthetic but
shape-faithful artifact — the same stage tree, receipt chain, manifest,
and digest links the committed producer emits — and drive one negative
per rejection family.
"""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
VERIFIER = REPO / "crates" / "opi-eval" / "scripts" / "verify-phase18-native-artifact.py"

WORKFLOW_PATH = ".github/workflows/phase18-native-smoke.yml"

CRITERIA = ("P18-A02", "P18-A03", "P18-A04", "P18-A08", "P18-A09",
            "P18-A10", "P18-A12", "BMK-003")

BENCHMARKS = {
    "terminal-bench-2.1": "terminal-bench-2.1",
    "terminal-bench-3.0": "terminal-bench-3.0",
    "deepswe-v1.1": "deepswe-v1.1",
}

CONFORMANCE_CASES = [
    ("agent", "opi", "completed"), ("agent", "opi", "identity"),
    ("agent", "pi", "completed"), ("agent", "pi", "identity"),
    ("benchmark", "terminal-bench-2.1", "completed"),
    ("benchmark", "terminal-bench-2.1", "identity"),
    ("benchmark", "terminal-bench-2.1", "immutable-capture"),
    ("benchmark", "terminal-bench-3.0", "completed"),
    ("benchmark", "terminal-bench-3.0", "identity"),
    ("benchmark", "terminal-bench-3.0", "immutable-capture"),
    ("benchmark", "deepswe-v1.1", "completed"),
    ("benchmark", "deepswe-v1.1", "identity"),
    ("benchmark", "deepswe-v1.1", "immutable-capture"),
]


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_json(path: Path, payload) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8")


def stage_receipt(stage: Path, name: str, detail) -> None:
    write_json(stage / name / "receipt.json",
               {"stage": name, "detail": detail})


def trial_receipt(trial: str, product: str, task: str, group: str,
                  reward: str = "known:1(verifier_reported)") -> dict:
    native: dict = {
        "schema": "phase18-trial-receipt/1",
        "id": trial,
        "subject": f"subject-{product}",
        "task": task,
        "group": group,
        "status": "sealed",
        "agent": {
            "product": product,
            "exit_state": "exited:0",
            "completion": "completed",
            "failure_kind": None,
            "boundary": None,
            "stdout_truncated": False,
            "stderr_truncated": False,
            "cleanup": "not-required",
            "stdout_bytes": 128,
            "stderr_bytes": 4,
        },
        "verifier": {
            "exit_state": "exited:0",
            "reward": reward,
            "completion": "verified",
            "failure_kind": None,
            "boundary": None,
        },
        "authority": {"dispatch": "attempted", "settle": "observed",
                      "grade-dispatch": "attempted", "grade-settle": "observed"},
        "bundle_identity": sha(f"bundle-{trial}".encode()),
        "pre_seal_digest": sha(f"preseal-{trial}".encode()),
        "seal_result": {"sealed": {"bundle_digest": sha(f"bundle-{trial}".encode())}},
    }
    return native


def build_trial(stage: Path, cfg: str, trial: str, product: str, task: str,
                group: str, reward: str = "known:1(verifier_reported)") -> dict:
    root = stage / "07-trials" / cfg / "trials" / trial
    receipt = trial_receipt(trial, product, task, group, reward)
    write_json(root / "receipt.json", receipt)
    bundle = root / "bundle"
    write_json(bundle / "intent.json",
               {"schema": "phase18-run-bundle-intent/1", "trial": trial})
    artifacts = bundle / "artifacts"
    for key, body in (("native/agent-stdout.log", f"{trial} stdout\n"),
                      ("native/agent-stderr.log", ""),
                      ("native/agent-answer.txt", "answer\n")):
        target = artifacts / key
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(body, encoding="utf-8")
    if product == "opi":
        for key, payload in (
                ("native/evidence/manifest",
                 json.dumps({"schema": "phase17-manifest/1", "correlation":
                             {"sequence": 1}, "outcome": "success"})),
                ("native/evidence/records",
                 json.dumps({"schema": "phase17-evidence/1", "records": 2}))):
            target = artifacts / key
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(payload + "\n", encoding="utf-8")
    else:
        target = artifacts / "native/events/stdout"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text('{"type":"agent_end"}\n', encoding="utf-8")
    write_json(root / "report.json",
               {"schema": "phase18-benchmark-report/1", "task": task,
                "product": product, "verdict": "graded",
                "metrics": {"passed": 1, "total": 1}})
    return receipt


def build_stage(tmp: Path, candidate: str) -> tuple[Path, Path, dict]:
    """Builds the synthetic sealed stage tree, tar, zip, and receipts."""
    stage = tmp / "stage"
    dispatch = {
        "candidate_sha": candidate,
        "github_workflow_ref": "refs/heads/main",
        "github_workflow_sha": candidate,
        "workflow_path": WORKFLOW_PATH,
        "workflow_sha256_read_from_workflow_sha": sha(b"workflow-bytes\n"),
        "checkout_head": candidate,
        "bound_scripts": {
            "producer": {"path": "crates/opi-eval/scripts/phase18-native-smoke.sh",
                         "role": "producer", "sha256": sha(b"producer\n")},
            "agent-builder": {"path": "crates/opi-eval/scripts/phase18-build-agent-artifacts.sh",
                              "role": "builder", "sha256": sha(b"builder\n")},
            "provider": {"path": "crates/opi-eval/scripts/phase18-scripted-provider.py",
                         "role": "provider", "sha256": sha(b"provider\n")},
            "verifier": {"path": "crates/opi-eval/scripts/verify-phase18-native-ci.py",
                         "role": "verifier", "sha256": sha(b"ci-verifier\n")},
        },
        "immutable_actions": [
            {"name": "actions/checkout", "version": "v4.2.2",
             "commit": "11bd71901bbe5b1630ceea73d27597364c9af683"},
            {"name": "dtolnay/rust-toolchain", "version": "1.97.0",
             "commit": "889fac408b4da0905346410f253f0c55fbcb6613"},
            {"name": "actions/setup-node", "version": "v4.4.0",
             "commit": "49933ea5288caeca8642d1e84afbd3f7d6820020"},
            {"name": "astral-sh/setup-uv", "version": "v6.7.0",
             "commit": "b75a909f75acd358c2196fb9a5f1299a9a8868a4"},
            {"name": "docker/setup-buildx-action", "version": "v3.11.1",
             "commit": "e468171a9de216ec08956ac3ada2f0791b6bd435"},
            {"name": "actions/upload-artifact", "version": "v4.6.2",
             "commit": "ea165f8d65b6e75b540449e92b4886f43607fa02"},
        ],
    }
    write_json(stage / "00-dispatch" / "dispatch.json", dispatch)
    stage_receipt(stage, "00-dispatch", {"dispatch_binding": dispatch})
    for name in ("01-identity", "02-tools", "03-external", "04-agents",
                 "05-provider"):
        stage_receipt(stage, name, {"ok": True})

    canary = stage / "06-canaries"
    markers = ["__P18_CANARY_openssl_selfsigned_cert__",
               "__P18_CANARY_batched_eval_parity__",
               "__P18_CANARY_abs_module_cache_flags__"]
    write_json(canary / "canary-preflight.json", {
        "oracle_manifest": [
            {"benchmark": key, "markers": [marker]}
            for (key, marker) in zip(sorted(BENCHMARKS), markers)],
        "probed_surfaces": ["agent-stdout", "agent-workspace"],
        "leakage": [],
        "negative_result_required": True,
        "negative_result_observed": True,
    })
    (canary / "canary-markers.txt").write_text(
        "".join(m + "\n" for m in markers), encoding="utf-8")
    stage_receipt(stage, "06-canaries", {"preflight": "negative"})

    material = stage / "06-material"
    material.mkdir(parents=True, exist_ok=True)
    lock_bytes = b"static-lock\n"
    (material / "external-lock.json").write_bytes(lock_bytes)
    wrappers = material / "wrappers"
    material_json = {
        "schema": "phase18-native-material/1",
        "static_lock": {"path": str(material / "external-lock.json"),
                        "sha256": sha(lock_bytes)},
        "provider": {
            "script": {"path": "crates/opi-eval/scripts/phase18-scripted-provider.py",
                       "sha256": sha(b"provider\n")},
            "endpoint": "http://127.0.0.1:48127/v1",
            "request_log": str(stage / "05-provider" / "requests.jsonl"),
        },
        "agents": {
            product: {
                "executable": {"path": str(wrappers / f"agent-{product}-generic.sh"),
                               "sha256": sha(f"agent-{product}\n".encode())},
                "model": "scripted:phase18" if product == "opi"
                         else "scripted:scripted/phase18",
                "provider_env": {"OPENAI_API_KEY": "<dummy-scripted-credential>"}
                                if product == "opi" else {"PI_API_KEY": "<redacted-dummy>"},
                "config": {"kind": "opi-toml" if product == "opi"
                           else "pi-models-json",
                           "base_url": "http://127.0.0.1:48127/v1",
                           "model_id": "phase18" if product == "opi"
                                       else "scripted/phase18",
                           "api_key": "<dummy-scripted-credential>"}
            }
            for product in ("opi", "pi")
        },
        "benchmarks": {},
    }
    for bench, short in BENCHMARKS.items():
        package = material / "packages" / short
        package.mkdir(parents=True, exist_ok=True)
        (package / "task.yaml").write_text("official: true\n", encoding="utf-8")
        digest = sha(b"task-package:" + short.encode())
        material_json["benchmarks"][bench] = {
            "profile": f"crates/opi-eval/profiles/benchmarks/{bench}.toml",
            "task_package": str(package),
            "task_package_manifest_sha256": digest,
            "verifier_executable": {"path": str(wrappers / f"verifier-{bench}.sh"),
                                    "sha256": sha(f"verifier-{bench}\n".encode())},
            "verifier_env": {},
            "oracle": {"path": str(wrappers / f"oracle-{bench}.sh"),
                       "sha256": sha(f"oracle-{bench}\n".encode())},
            "oracle_env": {},
        }
    write_json(material / "material.json", material_json)
    stage_receipt(stage, "06-material", {"material": True})
    write_json(material / "materialize-receipt.json",
               {"schema": "phase18-materialize-receipt/1", "wrappers": {}})

    conformance = material / "conformance"
    for suite, adapter, case in CONFORMANCE_CASES:
        root = conformance / "reports" / f"{suite}-{adapter}-{case}"
        write_json(root / "report.json", {"met": True, "case": case})
    write_json(conformance / "receipt.json",
               {"stage": "conformance-rerun",
                "detail": {"cases_run": len(CONFORMANCE_CASES),
                           "mode": "native-material"}})

    oracle = material / "oracle"
    for bench, short in BENCHMARKS.items():
        root = oracle / "runs" / short
        write_json(root / "run-report.json", {
            "schema": "phase18-run-report/1",
            "experiment": f"phase18-native-{short}",
            "outcome": "preflight-only",
            "preflight": {
                "schema": "phase18-oracle-preflight/1",
                "benchmark": bench,
                "task": f"{short}-official-task",
                "oracle_executable_sha256": sha(f"oracle-{bench}\n".encode()),
                "outcome": "passed",
            },
        })
        write_json(root / "preflight" / bench / "preflight-receipt.json", {
            "schema": "phase18-oracle-preflight/1",
            "benchmark": bench, "task": f"{short}-official-task",
            "oracle_executable_sha256": sha(f"oracle-{bench}\n".encode()),
            "outcome": "passed",
        })
    write_json(oracle / "receipt.json",
               {"stage": "oracle-preflight",
                "detail": {"configs_preflighted": len(BENCHMARKS)}})

    provider_log = stage / "05-provider" / "requests.jsonl"
    provider_log.parent.mkdir(parents=True, exist_ok=True)
    provider_log.write_text(
        "".join(json.dumps({"schema": "phase18-scripted-provider-log/1",
                            "request_sha256": sha(f"req-{i}".encode()),
                            "script": "phase18-scripted-provider/1"}) + "\n"
                for i in range(6)), encoding="utf-8")

    trials: list[dict] = []
    for bench, short in BENCHMARKS.items():
        cfg = short
        receipts = [
            build_trial(stage, cfg, f"{short}-opi", "opi",
                        f"{short}-official-task", short),
            build_trial(stage, cfg, f"{short}-pi", "pi",
                        f"{short}-official-task", short),
        ]
        trials.extend(receipts)
        write_json(stage / "07-trials" / cfg / "run-report.json", {
            "schema": "phase18-run-report/1",
            "experiment": f"phase18-native-{short}",
            "manifest_digest": sha(f"manifest-{short}".encode()),
            "integrity_digest": sha(f"integrity-{short}".encode()),
            "outcome": "completed",
            "trials": receipts,
            "pairs": [{
                "edge": "cross-agent",
                "task": f"{short}-official-task",
                "group": short,
                "baseline_trial": f"{short}-opi",
                "candidate_trial": f"{short}-pi",
                "comparability": "comparable",
            }],
        })
    stage_receipt(stage / "07-trials", "run-trials",
                  {"trials": len(trials), "sealed": len(trials)})

    return stage, None, dispatch


def seal(stage: Path, tmp: Path, expires_in_days: int = 90) -> dict:
    """Builds manifest, outer receipt, tar, zip, and upload receipt."""
    seal_out = stage / "08-seal"
    seal_out.mkdir(parents=True, exist_ok=True)
    manifest: dict = {}
    for path in sorted(stage.rglob("*")):
        if path.is_file() and seal_out not in path.parents:
            manifest[path.relative_to(stage).as_posix()] = sha(path.read_bytes())
    write_json(seal_out / "artifact-manifest.json",
               {"schema": "phase18-native-artifact-manifest/1", "files": manifest})
    outer = {
        "schema": "phase18-native-outer-receipt/1",
        "dispatch": json.loads(
            (stage / "00-dispatch" / "dispatch.json").read_text("utf-8")),
        "stage_receipts": [
            {"stage": json.loads(p.read_text("utf-8"))["stage"],
             "sha256": sha(p.read_bytes())}
            for p in sorted(stage.glob("*/receipt.json"))],
        "canary_preflight_negative_recorded": True,
        "artifact_manifest_sha256": sha(
            (seal_out / "artifact-manifest.json").read_bytes()),
        "conformance_evidence_only": True,
        "leaderboard_claim": "none",
    }
    write_json(seal_out / "outer-receipt.json", outer)

    tar_path = tmp / "sealed-artifact.tar"
    with tarfile.open(tar_path, "w") as archive:
        for path in sorted(stage.rglob("*")):
            if path.is_file():
                archive.add(path, arcname=path.relative_to(stage).as_posix(),
                            recursive=False)

    import datetime
    now = datetime.datetime.now(datetime.timezone.utc)
    upload = {
        "schema": "phase18-upload-identity-receipt/1",
        "artifact_id": "1234567890",
        "artifact_url": "https://github.com/OdradekAI/opi/actions/runs/1/"
                        "artifacts/1234567890",
        "artifact_digest": "",
        "run_id": "1",
        "run_url": "https://github.com/OdradekAI/opi/actions/runs/1",
        "sealed_manifest_sha256": outer["artifact_manifest_sha256"],
        "outer_receipt_sha256": sha((seal_out / "outer-receipt.json").read_bytes()),
        "recorded_at": now.isoformat(),
        "expires_at": (now + datetime.timedelta(days=expires_in_days)).isoformat(),
        "retention_days": expires_in_days,
    }
    return {"tar": tar_path, "outer": outer, "upload": upload}


def re_tar(stage: Path, tmp: Path) -> Path:
    """Re-archives the stage without recomputing the seal manifest."""
    tar_path = tmp / "sealed-artifact.tar"
    with tarfile.open(tar_path, "w") as archive:
        for path in sorted(stage.rglob("*")):
            if path.is_file():
                archive.add(path, arcname=path.relative_to(stage).as_posix(),
                            recursive=False)
    return tar_path


def package_zip(tar_path: Path, upload: dict) -> tuple[Path, dict]:
    zip_path = tar_path.with_suffix(".zip")
    with zipfile.ZipFile(zip_path, "w") as bundle:
        bundle.write(tar_path, arcname=tar_path.name)
    upload["artifact_digest"] = sha(zip_path.read_bytes())
    write_json(tar_path.parent / "upload-receipt.json", upload)
    return zip_path, upload


def build_git_repo(tmp: Path) -> tuple[Path, str]:
    """Mini git repository holding the bound bytes at the candidate."""
    repo = tmp / "repo"
    repo.mkdir(parents=True, exist_ok=True)
    run = lambda *args: subprocess.run(
        ["git", "-C", str(repo), *args], check=True, capture_output=True,
        text=True).stdout
    run("init", "-q")
    # The digest-to-bytes mapping is fixed by the fixture.
    bodies = {
        "crates/opi-eval/scripts/phase18-native-smoke.sh": b"producer\n",
        "crates/opi-eval/scripts/phase18-build-agent-artifacts.sh": b"builder\n",
        "crates/opi-eval/scripts/phase18-scripted-provider.py": b"provider\n",
        "crates/opi-eval/scripts/verify-phase18-native-ci.py": b"ci-verifier\n",
        WORKFLOW_PATH: b"workflow-bytes\n",
    }
    for rel, body in bodies.items():
        target = repo / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(body)
    run("-c", "user.email=t@example.com", "-c", "user.name=t",
        "add", "-A")
    run("-c", "user.email=t@example.com", "-c", "user.name=t",
        "commit", "-q", "-m", "candidate")
    return repo, run("rev-parse", "HEAD").strip()


class ArtifactVerifier(unittest.TestCase):
    def run_verifier(self, *args) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(VERIFIER), *map(str, args)],
            capture_output=True, text=True)

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)
        self.repo, self.candidate = build_git_repo(self.tmp)
        self.stage, _, self.dispatch = build_stage(self.tmp, self.candidate)
        self.sealed = seal(self.stage, self.tmp)
        self.zip_path, self.upload = package_zip(
            self.sealed["tar"], dict(self.sealed["upload"]))
        self.receipt_path = self.tmp / "upload-receipt.json"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def verify(self, criterion: str = "all-native", **kw) -> subprocess.CompletedProcess:
        return self.run_verifier(
            "--criterion", criterion,
            "--expected-commit",
            kw.get("commit", self.candidate),
            "--receipt", kw.get("receipt", self.receipt_path),
            "--artifact", kw.get("artifact", self.zip_path),
            "--repo", kw.get("repo", self.repo),
        )

    def test_all_native_accepts(self) -> None:
        result = self.verify()
        self.assertEqual(result.returncode, 0, result.stderr)
        for criterion in CRITERIA[:-1]:
            self.assertIn(f"{criterion} verified", result.stdout)

    def test_bare_tar_accepted(self) -> None:
        result = self.verify(
            "P18-A02", artifact=self.sealed["tar"])
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_each_criterion_accepts(self) -> None:
        for criterion in CRITERIA:
            with self.subTest(criterion=criterion):
                result = self.verify(criterion)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_expired_artifact_rejects(self) -> None:
        upload = dict(self.upload)
        upload["expires_at"] = "2020-01-01T00:00:00+00:00"
        write_json(self.receipt_path, upload)
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expiry", result.stderr)

    def test_wrong_expected_commit_rejects(self) -> None:
        result = self.verify(commit="0" * 40)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("candidate", result.stderr)

    def test_zip_digest_mismatch_rejects(self) -> None:
        # A different-but-valid hex digest means the downloaded bytes
        # are not the uploaded artifact (transport corruption): reject.
        upload = dict(self.sealed["upload"])
        upload["artifact_digest"] = "f" * 64
        write_json(self.receipt_path, upload)
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("digest", result.stderr)

    def test_malformed_artifact_digest_rejects(self) -> None:
        upload = dict(self.sealed["upload"])
        upload["artifact_digest"] = "not-a-digest"
        write_json(self.receipt_path, upload)
        result = self.verify()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("digest", result.stderr)

    def test_manifest_file_drift_rejects(self) -> None:
        target = self.stage / "07-trials" / "terminal-bench-2.1" / "trials" / "terminal-bench-2.1-opi" / "bundle" / \
            "artifacts" / "native" / "agent-stdout.log"
        target.write_text("tampered\n", encoding="utf-8")
        tar_path = re_tar(self.stage, self.tmp)
        zip_path, upload = package_zip(tar_path, dict(self.sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("manifest", result.stderr)

    def test_missing_trial_rejects(self) -> None:
        report_path = self.stage / "07-trials" / "deepswe-v1.1" / "run-report.json"
        report = json.loads(report_path.read_text("utf-8"))
        report["trials"] = report["trials"][:1]
        write_json(report_path, report)
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("trial", result.stderr)

    def test_positive_canary_rejects(self) -> None:
        canary = json.loads(
            (self.stage / "06-canaries" / "canary-preflight.json").read_text("utf-8"))
        canary["negative_result_observed"] = False
        canary["leakage"] = [canary["oracle_manifest"][0]["markers"][0]]
        write_json(self.stage / "06-canaries" / "canary-preflight.json", canary)
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("canary", result.stderr)

    def test_verifier_failed_rejects(self) -> None:
        root = self.stage / "07-trials" / "terminal-bench-3.0" / "trials" / "terminal-bench-3.0-pi"
        receipt = json.loads((root / "receipt.json").read_text("utf-8"))
        receipt["verifier"]["completion"] = "failed"
        receipt["verifier"]["failure_kind"] = "verifier-noncompletion"
        write_json(root / "receipt.json", receipt)
        report_path = self.stage / "07-trials" / "terminal-bench-3.0" / "run-report.json"
        report = json.loads(report_path.read_text("utf-8"))
        report["trials"] = [
            {**t, "verifier": dict(t["verifier"], completion="failed",
                                   failure_kind="verifier-noncompletion")}
            if t["id"] == "terminal-bench-3.0-pi" else t for t in report["trials"]]
        write_json(report_path, report)
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("verifier", result.stderr)

    def test_reward_zero_is_integration_result(self) -> None:
        for cfg in BENCHMARKS.values():
            report_path = self.stage / "07-trials" / cfg / "run-report.json"
            report = json.loads(report_path.read_text("utf-8"))
            for trial in report["trials"]:
                trial["verifier"]["reward"] = "known:0(verifier_reported)"
            write_json(report_path, report)
            for trial_id in (f"{cfg}-opi", f"{cfg}-pi"):
                root = self.stage / "07-trials" / cfg / "trials" / trial_id
                receipt = json.loads((root / "receipt.json").read_text("utf-8"))
                receipt["verifier"]["reward"] = "known:0(verifier_reported)"
                write_json(root / "receipt.json", receipt)
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("conformance-only", result.stdout)

    def test_missing_conformance_rerun_rejects(self) -> None:
        import shutil
        shutil.rmtree(self.stage / "06-material" / "conformance")
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("conformance", result.stderr)

    def test_missing_oracle_preflight_rejects(self) -> None:
        (self.stage / "06-material" / "oracle" / "runs" / "terminal-bench-3.0" /
         "preflight" / "terminal-bench-3.0" / "preflight-receipt.json").unlink()
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("oracle", result.stderr)

    def test_bound_script_drift_rejects(self) -> None:
        dispatch = json.loads(
            (self.stage / "00-dispatch" / "dispatch.json").read_text("utf-8"))
        dispatch["bound_scripts"]["producer"]["sha256"] = "0" * 64
        write_json(self.stage / "00-dispatch" / "dispatch.json", dispatch)
        sealed = seal(self.stage, self.tmp)
        zip_path, upload = package_zip(sealed["tar"], dict(sealed["upload"]))
        result = self.verify(artifact=zip_path)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("bound", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
