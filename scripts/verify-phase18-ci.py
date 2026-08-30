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
  python scripts/verify-phase18-ci.py --workflow .github/workflows/ci.yml
  python scripts/verify-phase18-ci.py --receipt <downloaded-receipt.json>

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


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--workflow")
    parser.add_argument("--receipt")
    args = parser.parse_args(argv)

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
