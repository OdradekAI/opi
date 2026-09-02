#!/usr/bin/env python3
"""Static and receipt verifier for the Phase 18 CI attestation contract
(task 18.16; consumed by task 18.16.1).

Static mode proves the repository CI workflow gained ONLY the minimal
attestation producer the Phase needs:

* every pre-existing pull-request merge-ref integration job survives
  untouched (the required-job set is exactly the pre-Phase set plus the
  one attestation job);
* the attestation job runs on all three platforms, checks out the
  ordinary merge ref, and records the pull-request head SEPARATELY in its
  receipt;
* no job substitutes head-only checkout semantics for merge-ref checks;
* every action is pinned by a full commit; and
* the producer records the receipt identity contract (schema, merge
  commit, PR head, workflow-bytes digest, required jobs, run identity).

Receipt mode validates one downloaded attestation receipt against the
same identity contract.

Usage:
  python crates/opi-eval/scripts/verify-phase18-ci.py --workflow .github/workflows/ci.yml
  python crates/opi-eval/scripts/verify-phase18-ci.py --receipt <downloaded-receipt.json>
  python crates/opi-eval/scripts/verify-phase18-ci.py --terminal --expected-head <sha> \
      --run-metadata <github-run.json> --jobs-metadata <github-jobs.json> \
      --artifact-metadata <github-artifact.json> \
      --inner-receipt <downloaded-inner-receipt.json> \
      --output docs/snapshots/phase18/ci-receipt.json

Exit 0 accepts; exit 1 prints one `finding <family>` line per violation.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

RECEIPT_SCHEMA = "phase18-ci-attestation/1"

# The integration checks that existed before Phase 18 added the attestation
# producer. Removing or renaming any of them replaces merge-ref integration
# semantics, which the Phase forbids.
PREEXISTING_JOBS = (
    "docs_contract",
    "fmt",
    "clippy",
    "test",
    "execution_acceptance",
    "phase17_acceptance",
    "doctest",
    "doc",
    "sandbox_package",
    "target_check",
)

ATTESTATION_JOB = "phase18_attestation"

REQUIRED_JOBS = PREEXISTING_JOBS + (ATTESTATION_JOB,)

# Actions the attestation job itself may use. The pre-existing jobs keep
# their own pin posture; this task owns only the new producer.
ALLOWED_ACTIONS = {
    ("actions/checkout", "11bd71901bbe5b1630ceea73d27597364c9af683"),
    ("actions/upload-artifact", "ea165f8d65b6e75b540449e92b4886f43607fa02"),
}

COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
USES_RE = re.compile(r"^\s*(?:-\s+)?uses:\s*(\S+)@(\S+)", re.MULTILINE)
# Job keys: exactly-two-space indented `name:` at job level.
JOB_KEY_RE = re.compile(r"^  ([a-z0-9_]+):$", re.MULTILINE)
# Any checkout of the pull-request head would substitute head-only
# semantics for merge-ref integration checks.
HEAD_ONLY_RE = re.compile(
    r"ref:\s*\$\{\{\s*github\.event\.pull_request(?:\.head)?(?:\.sha)?\s*\}\}")

RUNNER_OSES = {"Linux", "Windows", "macOS"}
EVENTS = {"push", "pull_request"}

# The runner reports the workflow's display names; map them back to the
# job keys whose set the receipt contract freezes. Sourced from
# .github/workflows/ci.yml's `name:` fields.
DISPLAY_TO_KEY = {
    "docs_contract": "docs_contract",
    "fmt": "fmt",
    "clippy": "clippy",
    "test": "test",
    "execution_acceptance": "execution_acceptance",
    "Phase 17 acceptance": "phase17_acceptance",
    "doctest": "doctest",
    "doc": "doc",
    "opi-sandbox package": "sandbox_package",
    "Target check": "target_check",
    "Phase 18 attestation": "phase18_attestation",
}


def verify_workflow_static(text: str) -> list[str]:
    rows: list[str] = []

    def reject(family: str, detail: str) -> None:
        rows.append(f"finding {family}: {detail}")

    # Job keys live strictly under the `jobs:` mapping; the two-space
    # trigger keys under `on:` must not be mistaken for jobs.
    jobs_block = text.split("\njobs:\n", 1)[-1]
    jobs = JOB_KEY_RE.findall(jobs_block)
    for job in PREEXISTING_JOBS:
        if job not in jobs:
            reject(
                "merge-ref-jobs",
                f"pre-existing integration job disappeared: {job}",
            )
    allowed = set(REQUIRED_JOBS)
    for job in jobs:
        if job not in allowed:
            reject(
                "merge-ref-jobs",
                f"unexpected additional job beyond the one attestation "
                f"producer: {job}",
            )
    if ATTESTATION_JOB not in jobs:
        reject("attestation", "attestation producer job is missing")
        return rows

    # The attestation producer must cover all three platforms.
    for os_name in ("ubuntu-latest", "windows-latest", "macos-latest"):
        if os_name not in text:
            reject("attestation", f"attestation matrix misses {os_name}")

    # The pull-request head must be recorded separately (as data), and no
    # job may check it out.
    if "github.event.pull_request.head.sha" not in text:
        reject(
            "attestation",
            "the pull-request head is not recorded separately in the receipt",
        )
    match = HEAD_ONLY_RE.search(text)
    if match:
        reject(
            "attestation",
            f"head-only checkout semantics are forbidden: {match.group(0)!r}",
        )

    # Ordinary merge-ref integration triggers must survive.
    if "pull_request:" not in text or "push:" not in text:
        reject("merge-ref-jobs", "push/pull_request triggers must survive")

    # The actions the attestation job itself uses stay pinned by full
    # commits from the admitted table (the job's own supply chain).
    attestation_block = text.split(f"  {ATTESTATION_JOB}:", 1)[-1]
    for uses in USES_RE.finditer(attestation_block):
        name, ref = uses.group(1), uses.group(2)
        if not COMMIT_RE.match(ref):
            reject("action", f"{name} is not pinned by a full commit: {ref}")
        elif (name, ref) not in ALLOWED_ACTIONS:
            reject("action", f"{name}@{ref} is not in the admitted table")

    # The producer's receipt must carry the complete identity contract.
    for token in (
        f'"{RECEIPT_SCHEMA}"',
        '"merge_commit"',
        '"pull_request_head"',
        '"workflow_sha256"',
        '"required_jobs"',
        '"run_id"',
    ):
        if token not in text:
            reject(
                "attestation",
                f"receipt identity contract is incomplete: {token} missing",
            )

    return rows


def verify_receipt_file(path: Path) -> list[str]:
    rows: list[str] = []

    def reject(family: str, detail: str) -> None:
        rows.append(f"finding {family}: {detail}")

    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as error:
        return [f"finding receipt: unreadable receipt: {error}"]

    if not isinstance(value, dict):
        return ["finding receipt: the receipt must be a JSON object"]
    if value.get("schema") != RECEIPT_SCHEMA:
        reject("receipt", f"unexpected schema: {value.get('schema')!r}")
        return rows

    run_id = value.get("run_id")
    if not isinstance(run_id, int) or run_id <= 0:
        reject("receipt", "run_id must be a positive integer")
    attempt = value.get("run_attempt")
    if not isinstance(attempt, int) or attempt <= 0:
        reject("receipt", "run_attempt must be a positive integer")

    event = value.get("event")
    if event not in EVENTS:
        reject("receipt", f"event must be one of {sorted(EVENTS)}: {event!r}")

    merge = value.get("merge_commit")
    if not isinstance(merge, str) or not COMMIT_RE.match(merge):
        reject("receipt", "merge_commit must be a 40-hex commit")

    head = value.get("pull_request_head")
    if event == "pull_request":
        if not isinstance(head, str) or not COMMIT_RE.match(head):
            reject("receipt", "the pull-request head is missing on a PR event")
        elif head == merge:
            reject(
                "receipt",
                "the recorded head equals the merge commit (head-only semantics)",
            )
    elif event == "push":
        if head is not None:
            reject("receipt", "a push event must not carry a pull-request head")

    attestation_commit = value.get("attestation_commit")
    if attestation_commit != merge:
        reject(
            "receipt",
            "the attestation commit must equal the merge-ref checkout commit",
        )

    workflow_digest = value.get("workflow_sha256")
    if not isinstance(workflow_digest, str) or not SHA256_RE.match(workflow_digest):
        reject("receipt", "the workflow digest must be a 64-hex sha256")

    if value.get("runner_os") not in RUNNER_OSES:
        reject("receipt", f"runner_os must be one of {sorted(RUNNER_OSES)}")

    required = value.get("required_jobs")
    if required != list(REQUIRED_JOBS):
        reject(
            "receipt",
            f"required-job set must be exactly {list(REQUIRED_JOBS)}",
        )

    workflow_path = value.get("workflow_path")
    if workflow_path != ".github/workflows/ci.yml":
        reject("receipt", f"unexpected workflow path: {workflow_path!r}")

    return rows


TERMINAL_SCHEMA = "phase18-ci-terminal-receipt/1"
RUN_CONCLUSIONS_OK = {"success"}
JOB_CONCLUSIONS_OK = {"success"}


def verify_terminal(
    expected_head: str,
    run: dict,
    jobs: dict,
    artifacts: dict,
    inner: dict,
    workflow_root: Path,
) -> tuple[list[str], dict | None]:
    """Bind the completed three-platform run to one durable receipt.

    Rejects missing, skipped, failed, cancelled, mismatched, expired,
    fork-only, and self-claimed evidence. The returned receipt is
    redacted: identities, digests, conclusions, and expiry only.
    """
    rows: list[str] = []

    def reject(family: str, detail: str) -> None:
        rows.append(f"finding {family}: {detail}")

    if not COMMIT_RE.match(expected_head):
        return [f"finding terminal: --expected-head must be a 40-hex commit: "
                f"{expected_head!r}"], None

    # Run metadata: completed, successful, same-event PR run bound to the
    # expected candidate head. A fork-only or push run never qualifies.
    if run.get("status") != "completed":
        reject("run", f"the run is not completed: {run.get('status')!r}")
    if run.get("conclusion") not in RUN_CONCLUSIONS_OK:
        reject("run", f"the run conclusion is {run.get('conclusion')!r}")
    event = run.get("event")
    if event != "pull_request":
        reject("run", f"a terminal receipt requires a pull_request event, "
                      f"got {event!r}")
    head_sha = run.get("head_sha", "")
    if head_sha != expected_head:
        reject("run", f"run head {head_sha[:12]} != expected "
                      f"{expected_head[:12]}")

    # Every job succeeded on its merge-ref checkout.
    job_rows = jobs.get("jobs") or []
    if not job_rows:
        reject("jobs", "the jobs metadata carries no jobs")
    bad: list[str] = []
    for job in job_rows:
        name = job.get("name", "<unnamed>")
        conclusion = job.get("conclusion")
        if conclusion not in JOB_CONCLUSIONS_OK:
            bad.append(f"{name}={conclusion}")
    if bad:
        reject("jobs", "not every job succeeded: " + ", ".join(bad))
    families = {DISPLAY_TO_KEY.get(str(row.get("name", "")).split(" (")[0])
                 for row in job_rows}
    missing = [job for job in REQUIRED_JOBS if job not in families]
    if missing:
        reject("jobs", f"required job families missing: {missing}")
    # On pull_request events every job's head_sha is the pull-request head:
    # one consistent identity, equal to the run's head_sha.
    heads = {str(job.get("head_sha", "")) for job in job_rows}
    checkout_identities = sorted(h for h in heads if COMMIT_RE.match(h))
    if event == "pull_request":
        if len(checkout_identities) != 1:
            reject("jobs", "pull_request jobs must share one head identity, "
                           f"found {checkout_identities}")
        elif checkout_identities[0] != expected_head:
            reject("jobs", "job head identity drifts from the expected head")

    # Artifact metadata: the attestation uploads exist and none expired.
    artifact_rows = artifacts.get("artifacts") or []
    attestation_artifacts = [row for row in artifact_rows
                             if str(row.get("name", "")).startswith(
                                 "phase18-attestation-")]
    if len(attestation_artifacts) != len(RUNNER_OSES):
        reject("artifact", "expected one attestation artifact per platform, "
                           f"found {len(attestation_artifacts)}")
    for row in attestation_artifacts:
        if row.get("expired"):
            reject("artifact", f"attestation artifact {row.get('id')} expired")
        digest = str(row.get("digest") or "")
        if not SHA256_RE.match(digest.removeprefix("sha256:")):
            reject("artifact", f"attestation artifact {row.get('id')} "
                               f"carries no sha256 digest")

    # The downloaded inner receipt: it must verify on its own and agree
    # with the run metadata (no self-claimed identity drift).
    inner_id = inner.get("_artifact_id")
    matched = [row for row in attestation_artifacts
               if row.get("id") == inner_id]
    if not matched:
        reject("inner", "the inner receipt's artifact is missing from the "
                        "run's artifact metadata")
    artifact = matched[0] if matched else {}
    if inner.get("run_id") != run.get("id"):
        reject("inner", "inner receipt run_id drifts from the run metadata")
    if inner.get("event") != event:
        reject("inner", "inner receipt event drifts from the run metadata")
    inner_merge = str(inner.get("merge_commit") or "")
    if not COMMIT_RE.match(inner_merge):
        reject("inner", "inner receipt merge commit must be a 40-hex commit")
    elif event == "pull_request" and inner_merge == expected_head:
        reject("inner", "the inner merge commit equals the pull-request head "
                        "(head-only semantics, not a merge-ref checkout)")
    inner_head = inner.get("pull_request_head")
    if event == "pull_request" and inner_head != expected_head:
        reject("inner", "inner receipt head drifts from the expected head")
    inner_digest = inner.get("workflow_sha256")
    workflow_path = inner.get("workflow_path", ".github/workflows/ci.yml")
    workflow_file = workflow_root / workflow_path
    try:
        actual_digest = __import__("hashlib").sha256(
            workflow_file.read_bytes()).hexdigest()
    except OSError as error:
        actual_digest = ""
        reject("inner", f"cannot read the workflow bytes: {error}")
    if inner_digest != actual_digest:
        reject("inner", "inner receipt workflow digest does not match the "
                        "candidate workflow bytes")
    if inner.get("required_jobs") != list(REQUIRED_JOBS):
        reject("inner", "inner receipt required-job set drifted")
    downloaded_digest = str(inner.get("_artifact_sha256") or "")
    if downloaded_digest != str(artifact.get("digest") or ""):
        reject("inner", "the downloaded artifact bytes do not match the "
                        "uploaded digest (single-stream download required)")

    if rows:
        return rows, None

    receipt = {
        "schema": TERMINAL_SCHEMA,
        "status": "verified",
        "run_id": run.get("id"),
        "run_attempt": run.get("run_attempt"),
        "event": event,
        "workflow_path": workflow_path,
        "workflow_sha256": actual_digest,
        "candidate_head": expected_head,
        "pull_request_head": inner_head,
        "checkout_identities": checkout_identities + [inner_merge],
        "inner": {
            "schema": inner.get("schema"),
            "runner_os": inner.get("runner_os"),
            "workflow_sha256": inner_digest,
            "attestation_commit": inner.get("attestation_commit"),
        },
        "artifact": {
            "id": artifact.get("id"),
            "name": artifact.get("name"),
            "digest": artifact.get("digest"),
            "download_verified": downloaded_digest == str(
                artifact.get("digest") or ""),
            "size_in_bytes": artifact.get("size_in_bytes"),
            "expires_at": artifact.get("expires_at"),
        },
        "conclusions": {
            "run": run.get("conclusion"),
            "jobs_total": len(job_rows),
            "jobs_all_success": not bad,
            "required_jobs": list(REQUIRED_JOBS),
        },
        "updated_at": run.get("updated_at"),
    }
    return [], receipt


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--workflow")
    parser.add_argument("--receipt")
    parser.add_argument("--terminal", action="store_true")
    parser.add_argument("--expected-head")
    parser.add_argument("--run-metadata")
    parser.add_argument("--jobs-metadata")
    parser.add_argument("--artifact-metadata")
    parser.add_argument("--inner-receipt")
    parser.add_argument("--output")
    parser.add_argument("--repo", default=".")
    args = parser.parse_args(argv)

    if args.terminal:
        wanted = [args.expected_head, args.run_metadata, args.jobs_metadata,
                  args.artifact_metadata, args.inner_receipt, args.output]
        if not all(wanted):
            parser.error("--terminal requires --expected-head, --run-metadata,"
                         " --jobs-metadata, --artifact-metadata, "
                         "--inner-receipt, and --output")
        try:
            run = json.loads(Path(args.run_metadata).read_text(encoding="utf-8"))
            jobs = json.loads(Path(args.jobs_metadata).read_text(encoding="utf-8"))
            artifacts = json.loads(
                Path(args.artifact_metadata).read_text(encoding="utf-8"))
            inner = json.loads(
                Path(args.inner_receipt).read_text(encoding="utf-8"))
        except (OSError, ValueError) as error:
            print(f"finding terminal: unreadable metadata: {error}",
                  file=sys.stderr)
            return 1
        rows, receipt = verify_terminal(
            args.expected_head, run, jobs, artifacts, inner,
            Path(args.repo),
        )
        if rows or receipt is None:
            for row in rows:
                print(row, file=sys.stderr)
            return 1
        out = Path(args.output)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(
            json.dumps(receipt, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )
        print(f"phase18-ci terminal receipt: wrote {out}")
        return 0

    if bool(args.workflow) == bool(args.receipt):
        parser.error("exactly one of --workflow or --receipt is required")

    if args.workflow:
        findings = verify_workflow_static(
            Path(args.workflow).read_text(encoding="utf-8")
        )
    else:
        findings = verify_receipt_file(Path(args.receipt))

    if findings:
        for row in findings:
            print(row, file=sys.stderr)
        return 1
    print("phase18-ci: PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
