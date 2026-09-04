#!/usr/bin/env python3
"""Static verifier for the opi-eval native-smoke producer contract.

Proves the committed producer contract without running any native trial:
the workflow is manually dispatched only, pinned to ubuntu-24.04 with an
explicit timeout and concurrency guard, every action is an immutable full
commit from the admitted table, the workflow binds the candidate_sha and
invokes exactly the committed producer/builder/provider files, the
producer hashes the workflow bytes from github.workflow_sha (never the
working tree), launches the provider through the canonical Python in
no-site mode with one pre-resolved endpoint on one internal Docker
network and a closed environment allowlist, proves positive and negative
reachability, runs the canary-oracle preflight over pinned oracle
material, builds both agents locked with compiler-artifact selection,
and uploads only the sealed artifact after redaction.

Usage:
  python crates/opi-eval/scripts/verify-native-smoke-ci.py \
    --workflow .github/workflows/opi-eval-native-smoke.yml \
    --script crates/opi-eval/scripts/native-smoke.sh \
    --build-script crates/opi-eval/scripts/build-agent-artifacts.sh \
    --provider crates/opi-eval/scripts/scripted-provider.py

Exit 0 accepts; exit 1 prints one `finding <family>` line per violation.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ALLOWED_ACTIONS = {
    ("actions/checkout", "11bd71901bbe5b1630ceea73d27597364c9af683"),
    ("dtolnay/rust-toolchain", "889fac408b4da0905346410f253f0c55fbcb6613"),
    ("actions/setup-node", "49933ea5288caeca8642d1e84afbd3f7d6820020"),
    ("astral-sh/setup-uv", "b75a909f75acd358c2196fb9a5f1299a9a8868a4"),
    ("docker/setup-buildx-action", "e468171a9de216ec08956ac3ada2f0791b6bd435"),
    ("actions/upload-artifact", "ea165f8d65b6e75b540449e92b4886f43607fa02"),
}

STDLIB_MODULES = {
    "argparse", "datetime", "hashlib", "http", "json", "os", "socket",
    "subprocess", "sys", "threading", "time", "platform", "pathlib",
    "__future__",
}

COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
USES_RE = re.compile(r"^\s*uses:\s*(\S+)@(\S+)", re.MULTILINE)
FORBIDDEN_TRIGGERS_RE = re.compile(r"^  (push|pull_request|schedule|workflow_call):",
                                   re.MULTILINE)
# Ambient credential reads: assignments that do not project the declared
# dummy values, plus any environment/credential-file read primitive. The
# declared dummy projections and forwarded isolation names are admitted.
CREDENTIAL_ENV_RE = re.compile(
    r"\b(OPENAI_API_KEY|ANTHROPIC_API_KEY|PI_API_KEY)=(?!<(?:dummy|redacted-dummy))")
CREDENTIAL_READ_RE = re.compile(
    r"(?:os\.environ(?:\.get)?|getenv\(|printenv[ \t]+)[\[(]?\s*[\"']?"
    r"(OPENAI_API_KEY|ANTHROPIC_API_KEY|PI_API_KEY)")
IMPORT_RE = re.compile(r"^\s*(?:import|from)\s+([A-Za-z_][A-Za-z0-9_.]*)",
                       re.MULTILINE)
WORKFLOW_PATH_ARG_RE = re.compile(r"--workflow-path[ \t]+([^\s\\]+)")


class Findings:
    def __init__(self) -> None:
        self.rows: list[str] = []

    def reject(self, family: str, detail: str) -> None:
        self.rows.append(f"finding {family}: {detail}")

    def require(self, family: str, haystack: str, needle: str,
                detail: str) -> None:
        if needle not in haystack:
            self.reject(family, detail)

    def forbid(self, family: str, haystack: str, needle: str,
               detail: str) -> None:
        if needle in haystack:
            self.reject(family, detail)

    def require_count(self, family: str, haystack: str, needle: str,
                      expected: int, detail: str) -> None:
        observed = haystack.count(needle)
        if observed != expected:
            self.reject(family, f"{detail} (observed {observed}, "
                                f"expected {expected})")


def verify_workflow(text: str, f: Findings) -> None:
    if "workflow_dispatch:" not in text:
        f.reject("trigger", "the workflow must be manually dispatched")
    match = FORBIDDEN_TRIGGERS_RE.search(text)
    if match:
        f.reject("trigger", f"forbidden trigger '{match.group(1)}:' is present")
    if "runs-on: ubuntu-24.04" not in text:
        f.reject("runner", "the job must run on ubuntu-24.04")
    if "timeout-minutes: 360" not in text:
        f.reject("timeout", "the job must declare timeout-minutes: 360")
    if "permissions:\n  contents: read" not in text:
        f.reject("permissions", "the workflow must grant contents: read only")
    if "cancel-in-progress: false" not in text:
        f.reject("concurrency", "the concurrency guard must not cancel runs")
    if "CANDIDATE: ${{ inputs.candidate_sha }}" not in text:
        f.reject("candidate", "the dispatch must bind inputs.candidate_sha")
    if "github.workflow_sha" not in text or "github.workflow_ref" not in text:
        f.reject("candidate", "the dispatch must record the workflow identity")
    for needle, detail in (
            ('workflow_path=${WORKFLOW_REF%%@*}',
             "the workflow path must be derived from github.workflow_ref"),
            ('"$GITHUB_REPOSITORY"/.github/workflows/*.yml',
             "the qualified workflow path must be bound to the repository"),
            ('workflow_path=${workflow_path#"$GITHUB_REPOSITORY/"}',
             "the workflow path must strip the exact repository prefix"),
    ):
        if needle not in text:
            f.reject("workflow-path", detail)
    workflow_path_args = WORKFLOW_PATH_ARG_RE.findall(text)
    if workflow_path_args != ['"$workflow_path"']:
        f.reject("workflow-path", "verify-dispatch must receive the single "
                 "workflow-ref-derived path")
    for uses in USES_RE.finditer(text):
        name, ref = uses.group(1), uses.group(2)
        if not COMMIT_RE.match(ref):
            f.reject("action", f"{name} is not pinned by a full commit: {ref}")
        elif (name, ref) not in ALLOWED_ACTIONS:
            f.reject("action", f"{name}@{ref} is not in the admitted table")
    for stage in ("verify-dispatch", "host-identity", "record-tools",
                  "fetch-external", "build-agents", "provider-up",
                  "provider-probe", "preflight-canaries",
                  "materialize-configs", "conformance-rerun",
                  "oracle-preflight", "run-trials", "seal-upload",
                  "record-upload-identity"):
        if f"crates/opi-eval/scripts/native-smoke.sh {stage}" not in text:
            family = ("canary-preflight" if stage == "preflight-canaries"
                      else "stage")
            f.reject(family, f"the workflow must invoke stage {stage}")
    if "crates/opi-eval/scripts/build-agent-artifacts.sh" not in text:
        f.reject("binding", "the workflow must bind the agent builder script")
    if "crates/opi-eval/scripts/scripted-provider.py" not in text:
        f.reject("binding", "the workflow must bind the checked-in provider")
    if ("path: ${{ runner.temp }}/opi-eval-native/08-seal/"
            "sealed-artifact.tar") not in text:
        f.reject("upload", "only the sealed artifact may be uploaded")
    if "if-no-files-found: error" not in text:
        f.reject("upload", "the upload must fail when the artifact is missing")


def verify_producer(text: str, f: Findings) -> None:
    # The native driving stages must consume the materialized manifest:
    # configs are pinned through the production validate entry, the
    # conformance rerun drives the exact executables, the oracle
    # preflight precedes trials, and trials carry the material plus the
    # declared canary markers into the pre-seal scan.
    f.require("materialize", text,
              '"--native-material", str(material_path)',
              "config materialization must pin digests via validate")
    f.require("materialize", text, "--preflight-only",
              "the oracle preflight must use the preflight-only entry")
    f.require("materialize", text, "--native-material \"$material\"",
              "the trial stage must consume the materialized manifest")
    f.require("canary-preflight", text, "--canaries",
              "trials must gate sealing on the declared canary markers")
    f.require("upload", text, "opi-eval-upload-identity-receipt/1",
              "the upload identity receipt schema must be pinned")
    f.require("upload", text, '"artifact_digest"',
              "the upload receipt must bind the artifact digest")
    # Workflow-byte binding: bytes are read from github.workflow_sha, not
    # from the mutable working tree or HEAD.
    f.require("workflow-sha", text,
              'git -C "$REPO_ROOT" show "${workflow_sha}:${workflow_path}"',
              "the workflow bytes must be hashed from the workflow SHA")
    # Provider isolation: canonical Python, no-site mode, one endpoint, one
    # internal network, closed environment allowlist.
    f.require("no-site", text, ' -I -S "$provider"',
              "the provider must run under python3 -I -S")
    f.require("environment", text, 'env -i PATH="/usr/bin:/bin"',
              "the provider environment must be closed with env -i")
    f.require_count("endpoint", text, "for port in (", 1,
                    "exactly one deterministic endpoint candidate list")
    f.require_count("endpoint", text, '--listen "', 1,
                    "exactly one listener endpoint may be launched")
    f.require("network", text, 'docker network create --internal "$network"',
              "the provider network must be internal (no egress)")
    f.require("negative-probe", text,
              "provider-probe: the endpoint answers on a non-loopback interface",
              "the provider must refuse every non-loopback surface")
    f.require("negative-probe", text,
              "docker network inspect \"$network\" --format '{{.Internal}}'",
              "the dedicated network must be verified internal")
    f.require("negative-probe", text, "ss -ltn",
              "undeclared host listeners must be probed absent")
    # External identities: commits only, never mutable refs.
    f.forbid("mutable", text, " --branch ",
             "clones must not resolve a mutable branch or tag")
    f.forbid("mutable", text, '"main"', "no mutable 'main' identity")
    # Locked builds everywhere.
    f.require("locked", text, "cargo build --locked --release -p opi-eval",
              "the opi-eval producer build must be locked")
    f.require("locked", text, "cargo build --locked --release",
              "agent builds must be locked release builds")
    # Canary-oracle preflight over pinned oracle material.
    f.require("oracle", text,
              'if not pinned["path"].startswith(("solution/", "tests/")):',
              "the preflight must pin solution/ and tests/ oracle material")
    f.require("oracle", text, '"markers"',
              "the preflight must probe verbatim canary markers")
    # No ambient credential reads anywhere in the producer; only the
    # declared dummy projections and forwarded isolation names appear.
    credential = CREDENTIAL_ENV_RE.search(text)
    if credential:
        f.reject("credential",
                 f"ambient credential '{credential.group(1)}' is read")
    ambient_read = CREDENTIAL_READ_RE.search(text)
    if ambient_read:
        f.reject("credential",
                 f"ambient environment read '{ambient_read.group(0)}'")


def verify_builder(text: str, f: Findings) -> None:
    f.require("locked", text, "$(cargo build --locked --release",
              "the opi build invocation must be --locked --release")
    f.require("locked", text,
              "printf 'cargo build --locked --release -p opi-coding-agent "
              "--bin opi '",
              "the recorded build command must stay locked")
    f.require("compiler-artifact", text,
              '"compiler-artifact"',
              "the executable must be selected from a compiler-artifact")
    f.require("compiler-artifact", text,
              'if target.get("name") == "opi" and executable:',
              "the opi target must be selected by name with an executable")
    f.forbid("compiler-artifact", text, "target/release/opi",
             "the build must never assume a target/release path")
    f.require("compiler-artifact", text,
              "--message-format=json-render-diagnostics",
              "the build must use a machine-readable artifact stream")
    for token, detail in (
            ('run(["file", "-b", executable])', "file(1) identity"),
            ('run(["ldd", executable])', "ldd identity"),
            ('["rustc", "-Vv"]', "rustc -Vv identity"),
            ('["cargo", "-V"]', "cargo -V identity"),
            ('"$OPi_SOURCE/Cargo.lock"', "Cargo.lock identity"),
    ):
        if token in ('"$OPi_SOURCE/Cargo.lock"',):
            token = '"$OPI_SOURCE/Cargo.lock"'
        f.require("identity", text, token, f"missing {detail}")
    f.require("npm-ci", text, "npm ci --ignore-scripts",
              "pi must be installed with npm ci --ignore-scripts")
    f.forbid("npm-ci", text, "npm install",
             "npm install is not a locked install")
    f.require("bundle", text, "npm run build",
              "pi must be built from source")
    f.require("bundle", text, "test -f packages/coding-agent/dist/bundle/cli.js",
              "the pi bundle must be located by its canonical path")
    f.require("identity", text, '"shrinkwrap_sha256"',
              "the shrinkwrap digest must be recorded")
    f.require("identity", text, '"installed_tree_sha256"',
              "the installed tree digest must be recorded")
    f.require("identity", text, '"bundle_sha256"',
              "the bundle digest must be recorded")
    credential = CREDENTIAL_ENV_RE.search(text)
    if credential:
        f.reject("credential",
                 f"ambient credential '{credential.group(1)}' is read")
    ambient_read = CREDENTIAL_READ_RE.search(text)
    if ambient_read:
        f.reject("credential",
                 f"ambient environment read '{ambient_read.group(0)}'")


def verify_provider(text: str, f: Findings) -> None:
    if 'SCHEMA = "opi-eval-scripted-provider/1"' not in text:
        f.reject("provider", "the provider identity schema token is missing")
    for imported in IMPORT_RE.findall(text):
        root = imported.split(".")[0]
        if root not in STDLIB_MODULES:
            f.reject("provider",
                     f"the provider must stay stdlib-only, found '{root}'")
            break
    if "OPENAI_API_KEY" in text or "ANTHROPIC_API_KEY" in text:
        f.reject("provider", "the provider must not read ambient credentials")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--workflow", required=True)
    parser.add_argument("--script", required=True)
    parser.add_argument("--build-script", required=True)
    parser.add_argument("--provider", required=True)
    args = parser.parse_args()

    findings = Findings()
    paths = {"workflow": Path(args.workflow), "script": Path(args.script),
             "build-script": Path(args.build_script),
             "provider": Path(args.provider)}
    for label, path in paths.items():
        if not path.is_file():
            findings.reject(label, f"required file is missing: {path}")
    if findings.rows:
        for row in findings.rows:
            print(row)
        return 1

    workflow = paths["workflow"].read_text(encoding="utf-8")
    producer = paths["script"].read_text(encoding="utf-8")
    builder = paths["build-script"].read_text(encoding="utf-8")
    provider = paths["provider"].read_text(encoding="utf-8")

    verify_workflow(workflow, findings)
    verify_producer(producer, findings)
    verify_builder(builder, findings)
    verify_provider(provider, findings)

    if findings.rows:
        for row in findings.rows:
            print(row)
        return 1
    print("native-smoke-ci: producer contract verified "
          f"({len(ALLOWED_ACTIONS)} admitted actions, workflow {args.workflow})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
