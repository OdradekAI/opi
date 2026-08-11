# Opi Local Eval Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the explicit `opi-eval` local-smoke workflow into a deterministic, replayable, privacy-bounded evaluation foundation with reviewed cases and calibrated subjective diagnostics.

**Architecture:** Keep the capability inside `.claude/skills/opi-eval`. A single standard-library Python entry point validates TOML cases, runs isolated trials, seals bounded evidence, grades saved bundles offline, validates machine labels against calibration records, and renders small reports. Deterministic outcome and trajectory graders remain authoritative; the skill host, not the Python tool, dispatches an optional readonly LLM evaluator.

**Tech Stack:** Python 3.11+ standard library (`argparse`, `dataclasses`, `hashlib`, `json`, `pathlib`, `re`, `subprocess`, `tempfile`, `tomllib`, `unittest`), Opi NDJSON schema v2, TOML case manifests, Markdown documentation.

---

## Repository execution gates

- This plan is an input to the repository's canonical `opi-implement plan`
  admission gate. Do not execute implementation tasks until the user explicitly
  invokes that project-local skill and the work is admitted to the canonical
  ledger.
- The working tree already contains unrelated user changes. Every task must
  inspect `git status --short`, edit only the listed task-owned paths, and leave
  unrelated paths untouched.
- Commit steps below are conditional. Run them only after the user explicitly
  authorizes commits. Otherwise record the checkpoint and continue without
  staging.
- Real-provider `run` commands consume credentials and credits. Automated tests
  and plan verification use fake executables and saved bundles only.

## File map

| Path | Responsibility |
|---|---|
| `.claude/skills/opi-eval/scripts/opi_eval.py` | Private implementation and CLI for validation, execution, capture, grading, calibration, and reporting. |
| `.claude/skills/opi-eval/scripts/test_opi_eval.py` | Standard-library unit and fixture-backed integration tests. |
| `.claude/skills/opi-eval/cases/*.toml` | Authoritative versioned local-smoke cases. |
| `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/**` | Small redacted bundle proving deterministic offline regrading. |
| `.claude/skills/opi-eval/calibration/README.md` | Gold-label and machine-label schemas plus calibration policy. |
| `.claude/skills/opi-eval/SKILL.md` | Explicit orchestration contract, including real-credit and readonly-evaluator guardrails. |
| `.claude/skills/opi-eval/references/test-cases.md` | Case-authoring guide and manifest index; no duplicated prompts. |
| `.claude/skills/opi-eval/references/evaluator-prompt.md` | Atomic machine-label protocol for bounded evidence projections. |
| `.claude/skills/opi-eval/references/report-template.md` | Authority-separated report format. |
| `docs/eval/README.md` | Versioned report/history contract and artifact-retention rules. |

### Task 1: Add the case manifest parser and migrate the three cases

**Files:**
- Create: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Create: `.claude/skills/opi-eval/scripts/test_opi_eval.py`
- Create: `.claude/skills/opi-eval/cases/candy.toml`
- Create: `.claude/skills/opi-eval/cases/tool_chain.toml`
- Create: `.claude/skills/opi-eval/cases/context_retention.toml`
- Read: `.claude/skills/opi-eval/references/test-cases.md:1-161`

- [ ] **Step 1: Write failing parser and registry tests**

Create `test_opi_eval.py` with an import helper and these tests:

```python
from __future__ import annotations

import dataclasses
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock


MODULE_PATH = Path(__file__).with_name("opi_eval.py")
SPEC = importlib.util.spec_from_file_location("opi_eval", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
opi_eval = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = opi_eval
SPEC.loader.exec_module(opi_eval)


class ManifestTests(unittest.TestCase):
    def test_loads_pinned_tool_chain_case(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        self.assertEqual(case.id, "tool_chain")
        self.assertEqual(case.status, "pinned")
        self.assertEqual(case.capture_paths, ("result.txt",))
        self.assertIn("file_text_equals", [item.kind for item in case.assertions])

    def test_pinned_case_requires_review_and_required_outcome(self) -> None:
        manifest = textwrap.dedent("""
            schema_version = 1
            id = "bad_case"
            version = 1
            suite = "local-smoke"
            status = "pinned"
            risk_class = "read-only"
            capture_paths = []
            [source]
            kind = "manual"
            reference = "unit-test"
            sha256 = "0db52f4076c082518412afd3dd3576e2cb0c63703fd7fed5e23ade60efef31d9"
            [execution]
            tool_profile = "none"
            timeout_seconds = 30
            default_trials = 1
            minimum_pass_rate = 1.0
            [task]
            prompt = "answer"
            expected_behavior = "answer correctly"
        """)
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad_case.toml"
            path.write_text(manifest, encoding="utf-8")
            with self.assertRaisesRegex(opi_eval.EvalError, "review"):
                opi_eval.load_case(path)

    def test_rejects_workspace_escape_capture_path(self) -> None:
        source = (Path(__file__).parents[1] / "cases" / "tool_chain.toml").read_text(
            encoding="utf-8"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "tool_chain.toml"
            path.write_text(
                source.replace('capture_paths = ["result.txt"]',
                               'capture_paths = ["../result.txt"]'),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(opi_eval.EvalError, "workspace-relative"):
                opi_eval.load_case(path)

    def test_registry_rejects_duplicate_case_id(self) -> None:
        case_path = Path(__file__).parents[1] / "cases" / "candy.toml"
        case = opi_eval.load_case(case_path)
        with self.assertRaisesRegex(opi_eval.EvalError, "duplicate case id"):
            opi_eval.validate_registry([case, case])

    def test_rejects_source_digest_that_does_not_match_prompt(self) -> None:
        source = (Path(__file__).parents[1] / "cases" / "candy.toml").read_text(
            encoding="utf-8"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candy.toml"
            path.write_text(
                source.replace(
                    'sha256 = "8527df9edae53dde4cb32bb8fbe7f671ce905e2ddf81290543f86a4b6a9304cc"',
                    'sha256 = "0000000000000000000000000000000000000000000000000000000000000000"',
                ),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(opi_eval.EvalError,
                                        "does not match task prompt"):
                opi_eval.load_case(path)

    def test_registry_rejects_duplicate_source_digest(self) -> None:
        case_path = Path(__file__).parents[1] / "cases" / "candy.toml"
        case = opi_eval.load_case(case_path)
        duplicate = dataclasses.replace(
            case,
            id="candy_copy",
            source={**case.source, "reference": "unit-test-copy"},
        )
        with self.assertRaisesRegex(opi_eval.EvalError,
                                    "duplicate case source digest"):
            opi_eval.validate_registry([case, duplicate])

    def test_retired_case_requires_retirement_metadata(self) -> None:
        source = (Path(__file__).parents[1] / "cases" / "candy.toml").read_text(
            encoding="utf-8"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candy.toml"
            path.write_text(
                source.replace('status = "pinned"', 'status = "retired"'),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(opi_eval.EvalError,
                                        "retirement metadata"):
                opi_eval.load_case(path)

    def test_rejects_email_as_reviewer_identifier(self) -> None:
        source = (Path(__file__).parents[1] / "cases" / "candy.toml").read_text(
            encoding="utf-8"
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candy.toml"
            path.write_text(
                source.replace("original-case-author", "owner@example.com"),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(opi_eval.EvalError,
                                        "stable project identifiers"):
                opi_eval.load_case(path)


if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 2: Run the parser tests and verify the red state**

Run:

```text
python -m unittest .claude/skills/opi-eval/scripts/test_opi_eval.py
```

Expected: import fails because `opi_eval.py` and the case manifests do not yet
exist.

- [ ] **Step 3: Implement immutable manifest types and validation**

Create `opi_eval.py` with these public-in-file types and helpers. Keep all other
helpers private by prefixing them with `_`.

```python
#!/usr/bin/env python3
"""Deterministic local evaluation support for the explicit opi-eval skill."""

from __future__ import annotations

import argparse
import dataclasses
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid
from datetime import UTC, datetime
from pathlib import Path, PurePosixPath
from typing import Any, Iterable


CASE_SCHEMA_VERSION = 1
RUN_SCHEMA_VERSION = 1
GRADE_SCHEMA_VERSION = 1
MAX_CAPTURE_BYTES = 1_048_576
MAX_NDJSON_BYTES = 8_388_608
MAX_STDERR_BYTES = 65_536
CASE_ID_RE = re.compile(r"^[a-z][a-z0-9_]*$")
REVIEW_ID_RE = re.compile(r"^[a-z][a-z0-9_-]*$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
UTC_TIMESTAMP_RE = re.compile(
    r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z$"
)
CASE_STATUSES = {"candidate", "pinned", "retired"}
RISK_CLASSES = {"read-only", "workspace-write"}
TOOL_PROFILES = {"none", "mutating"}
SOURCE_KINDS = {"manual", "issue", "eval-finding", "session-export"}
PRIVACY_STATUSES = {"pending", "approved", "rejected"}
SEVERITIES = {"required", "advisory"}
ASSERTION_KINDS = {
    "exit_code_equals",
    "final_text_regex",
    "file_exists",
    "file_text_equals",
    "tool_call_sequence",
    "tool_call_count",
    "no_tool_error",
    "max_retry_count",
}


class EvalError(ValueError):
    """A stable, user-facing local-eval validation or grading failure."""


@dataclasses.dataclass(frozen=True)
class AssertionSpec:
    id: str
    kind: str
    severity: str
    options: dict[str, Any]


@dataclasses.dataclass(frozen=True)
class SubjectiveRubric:
    id: str
    version: int
    severity: str
    question: str
    evidence: tuple[str, ...]
    calibration: dict[str, Any]


@dataclasses.dataclass(frozen=True)
class CaseSpec:
    path: Path
    digest: str
    id: str
    version: int
    suite: str
    status: str
    risk_class: str
    capture_paths: tuple[str, ...]
    source: dict[str, Any]
    review: dict[str, Any] | None
    retirement: dict[str, Any] | None
    execution: dict[str, Any]
    fixtures: tuple[dict[str, str], ...]
    prompt: str
    user_value: str
    expected_behavior: str
    assertions: tuple[AssertionSpec, ...]
    subjective_rubrics: tuple[SubjectiveRubric, ...]


def _canonical_bytes(value: Any) -> bytes:
    return (json.dumps(value, ensure_ascii=False, sort_keys=True,
                       separators=(",", ":")) + "\n").encode("utf-8")


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise EvalError(message)


def _safe_relative_path(value: str) -> str:
    path = PurePosixPath(value.replace("\\", "/"))
    _require(bool(value) and not path.is_absolute(),
             f"path must be workspace-relative: {value!r}")
    _require(".." not in path.parts and ":" not in path.parts[0],
             f"path must be workspace-relative: {value!r}")
    normalized = path.as_posix()
    _require(normalized not in {"", "."},
             f"path must name a file: {value!r}")
    return normalized


def _parse_assertion(raw: dict[str, Any]) -> AssertionSpec:
    assertion_id = str(raw.get("id", ""))
    kind = str(raw.get("kind", ""))
    severity = str(raw.get("severity", ""))
    _require(bool(assertion_id), "assertion id is required")
    _require(kind in ASSERTION_KINDS, f"unsupported assertion kind: {kind}")
    _require(severity in SEVERITIES, f"unsupported assertion severity: {severity}")
    options = {key: value for key, value in raw.items()
               if key not in {"id", "kind", "severity"}}
    if "path" in options:
        options["path"] = _safe_relative_path(str(options["path"]))
    if kind == "final_text_regex":
        _require(isinstance(options.get("pattern"), str), "pattern must be text")
        try:
            re.compile(str(options["pattern"]))
        except re.error as error:
            raise EvalError(f"invalid final_text_regex: {error.msg}") from error
    if kind in {"exit_code_equals", "max_retry_count"}:
        key = "expected" if kind == "exit_code_equals" else "maximum"
        _require(isinstance(options.get(key), int), f"{kind} requires integer {key}")
    if kind == "file_exists":
        _require(isinstance(options.get("path"), str), "file_exists requires path")
        _require(isinstance(options.get("expected"), bool),
                 "file_exists requires boolean expected")
    if kind == "file_text_equals":
        _require(isinstance(options.get("path"), str), "file_text_equals requires path")
        _require(isinstance(options.get("expected"), str),
                 "file_text_equals requires text expected")
        _require(isinstance(options.get("allow_trailing_newline", False), bool),
                 "allow_trailing_newline must be boolean")
    if kind == "tool_call_sequence":
        _require(options.get("mode") == "ordered-subsequence",
                 "tool_call_sequence mode must be ordered-subsequence")
        _require(isinstance(options.get("expected"), list)
                 and all(isinstance(value, str) for value in options["expected"]),
                 "tool_call_sequence expected must be a string array")
    if kind == "tool_call_count":
        _require(isinstance(options.get("minimum"), int)
                 and isinstance(options.get("maximum"), int)
                 and 0 <= options["minimum"] <= options["maximum"],
                 "tool_call_count requires ordered non-negative bounds")
    if kind == "no_tool_error":
        _require(options.get("expected") is True,
                 "no_tool_error expected must be true")
    return AssertionSpec(assertion_id, kind, severity, options)


def load_case(path: Path) -> CaseSpec:
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, tomllib.TOMLDecodeError) as error:
        raise EvalError(
            f"cannot load case {path.name}: {type(error).__name__}"
        ) from error
    _require(raw.get("schema_version") == CASE_SCHEMA_VERSION,
             f"unsupported case schema in {path}")
    case_id = str(raw.get("id", ""))
    _require(bool(CASE_ID_RE.fullmatch(case_id)), f"invalid case id: {case_id!r}")
    _require(path.name == f"{case_id}.toml",
             f"case filename must be {case_id}.toml")
    version = raw.get("version")
    _require(isinstance(version, int) and version > 0, "case version must be positive")
    status = str(raw.get("status", ""))
    _require(status in CASE_STATUSES, f"invalid case status: {status}")
    _require(bool(str(raw.get("suite", ""))), "case suite is required")
    risk_class = str(raw.get("risk_class", ""))
    _require(risk_class in RISK_CLASSES, f"invalid risk class: {risk_class}")
    capture_paths = tuple(_safe_relative_path(str(item))
                          for item in raw.get("capture_paths", []))
    source = dict(raw.get("source", {}))
    _require(source.get("kind") in SOURCE_KINDS, "invalid source kind")
    _require(bool(source.get("reference")), "source reference is required")
    _require(bool(SHA256_RE.fullmatch(str(source.get("sha256", "")))),
             "source sha256 must be 64 lowercase hexadecimal characters")
    review = dict(raw["review"]) if "review" in raw else None
    if review is not None:
        owner = str(review.get("rubric_owner", ""))
        reviewers = review.get("reviewers", [])
        _require(bool(REVIEW_ID_RE.fullmatch(owner)),
                 "review rubric_owner must be a stable project identifier")
        _require(isinstance(reviewers, list) and len(reviewers) >= 2,
                 "review requires at least two reviewers")
        _require(all(isinstance(value, str) and REVIEW_ID_RE.fullmatch(value)
                     for value in reviewers),
                 "reviewers must be stable project identifiers")
        _require(len(reviewers) == len(set(reviewers)),
                 "reviewers must be unique")
        _require(bool(UTC_TIMESTAMP_RE.fullmatch(str(review.get("reviewed_at", "")))),
                 "reviewed_at must be a UTC timestamp")
        _require(review.get("privacy_status") in PRIVACY_STATUSES,
                 "invalid review privacy_status")
    if status in {"pinned", "retired"}:
        _require(review is not None, f"{status} case requires review")
        _require(review.get("privacy_status") == "approved",
                 "review privacy_status must be approved")
    retirement = dict(raw["retirement"]) if "retirement" in raw else None
    if status == "retired":
        _require(retirement is not None, "retired case requires retirement metadata")
        _require(bool(retirement.get("reason")), "retirement reason is required")
        _require(bool(UTC_TIMESTAMP_RE.fullmatch(
                     str(retirement.get("retired_at", "")))),
                 "retired_at must be a UTC timestamp")
        replacement = retirement.get("replacement_case")
        _require(replacement is None or bool(CASE_ID_RE.fullmatch(str(replacement))),
                 "replacement_case must be a case id")
    else:
        _require(retirement is None, "retirement metadata requires retired status")
    execution = dict(raw.get("execution", {}))
    _require(execution.get("tool_profile") in TOOL_PROFILES, "invalid tool profile")
    _require(isinstance(execution.get("timeout_seconds"), int)
             and execution["timeout_seconds"] > 0, "timeout must be positive")
    _require(isinstance(execution.get("default_trials"), int)
             and execution["default_trials"] > 0, "default_trials must be positive")
    minimum_pass_rate = execution.get("minimum_pass_rate")
    _require(isinstance(minimum_pass_rate, (int, float))
             and 0.0 <= float(minimum_pass_rate) <= 1.0,
             "minimum_pass_rate must be in 0.0..1.0")
    fixtures = tuple(dict(item) for item in raw.get("fixtures", {}).get("files", []))
    for fixture in fixtures:
        fixture["path"] = _safe_relative_path(str(fixture.get("path", "")))
        _require(isinstance(fixture.get("content"), str), "fixture content must be text")
        _require(len(fixture["content"].encode("utf-8")) <= MAX_CAPTURE_BYTES,
                 "fixture exceeds size limit")
    task = dict(raw.get("task", {}))
    _require(bool(task.get("prompt")), "task prompt is required")
    _require(bool(task.get("user_value")), "task user_value is required")
    _require(bool(task.get("expected_behavior")), "expected_behavior is required")
    prompt = str(task["prompt"])
    prompt_digest = hashlib.sha256(prompt.encode("utf-8")).hexdigest()
    _require(source["sha256"] == prompt_digest,
             "source sha256 does not match task prompt")
    assertions = tuple(_parse_assertion(dict(item)) for item in raw.get("assertions", []))
    rubric_ids = [item.id for item in assertions]
    subjective_values = []
    for item in raw.get("subjective_rubrics", []):
        calibration = dict(item.get("calibration", {}))
        rubric = SubjectiveRubric(
            id=str(item.get("id", "")),
            version=int(item.get("version", 0)),
            severity=str(item.get("severity", "")),
            question=str(item.get("question", "")),
            evidence=tuple(str(value) for value in item.get("evidence", [])),
            calibration=calibration,
        )
        _require(bool(rubric.id) and rubric.version > 0,
                 "subjective rubric requires id and positive version")
        _require(rubric.severity in SEVERITIES,
                 "subjective rubric severity is invalid")
        _require(bool(rubric.question) and bool(rubric.evidence),
                 "subjective rubric requires question and evidence")
        _require(isinstance(calibration.get("minimum_samples"), int)
                 and calibration["minimum_samples"] > 0,
                 "subjective rubric requires positive minimum_samples")
        for key in ("minimum_human_human_exact",
                    "minimum_human_machine_exact",
                    "maximum_unknown_rate"):
            _require(isinstance(calibration.get(key), (int, float))
                     and 0.0 <= float(calibration[key]) <= 1.0,
                     f"subjective rubric {key} must be in 0.0..1.0")
        _require(bool(SHA256_RE.fullmatch(
                     str(calibration.get("evaluator_fingerprint", "")))),
                 "subjective rubric requires evaluator_fingerprint")
        subjective_values.append(rubric)
    subjective = tuple(subjective_values)
    rubric_ids.extend(item.id for item in subjective)
    _require(len(rubric_ids) == len(set(rubric_ids)), "rubric ids must be unique")
    if status == "pinned":
        _require(any(item.severity == "required" and
                     item.kind in {"exit_code_equals", "final_text_regex",
                                   "file_exists", "file_text_equals"}
                     for item in assertions),
                 "pinned case requires a required outcome assertion")
    digest = hashlib.sha256(_canonical_bytes(raw)).hexdigest()
    return CaseSpec(
        path=path.resolve(), digest=digest, id=case_id, version=version,
        suite=str(raw["suite"]), status=status, risk_class=risk_class,
        capture_paths=capture_paths, source=source, review=review,
        retirement=retirement,
        execution=execution, fixtures=fixtures, prompt=prompt,
        user_value=str(task["user_value"]),
        expected_behavior=str(task["expected_behavior"]), assertions=assertions,
        subjective_rubrics=subjective,
    )


def validate_registry(cases: Iterable[CaseSpec]) -> tuple[CaseSpec, ...]:
    values = tuple(cases)
    ids = [case.id for case in values]
    if len(ids) != len(set(ids)):
        raise EvalError("duplicate case id")
    source_refs = [(case.source["kind"], case.source["reference"])
                   for case in values]
    if len(source_refs) != len(set(source_refs)):
        raise EvalError("duplicate case source reference")
    source_digests = [case.source["sha256"] for case in values]
    if len(source_digests) != len(set(source_digests)):
        raise EvalError("duplicate case source digest")
    known_ids = set(ids)
    for case in values:
        if case.retirement is None:
            continue
        replacement = case.retirement.get("replacement_case")
        if replacement is not None and replacement not in known_ids:
            raise EvalError(f"unknown replacement case: {replacement}")
        if replacement == case.id:
            raise EvalError("retired case cannot replace itself")
    return tuple(sorted(values, key=lambda case: case.id))
```

- [ ] **Step 4: Add the three authoritative TOML manifests**

For each manifest, move the corresponding `### Prompt` fenced block from
`references/test-cases.md` byte-for-byte into `task.prompt`. Set `source.sha256`
to the SHA-256 digest of the exact UTF-8 bytes in that `task.prompt`; this keeps
the source proof valid after the prose reference file is replaced. Use these
case-specific source values:

Use a TOML multiline basic string whose closing `"""` immediately follows the
last prompt character; do not introduce a terminal newline. The parser's
source-digest check is the authoritative verification of byte equivalence.

```toml
# candy.toml
[source]
kind = "manual"
reference = "git:c4994d4a177d9aee9c4713bef46fad2367e0cf1e:.claude/skills/opi-eval/references/test-cases.md#case-1-candy"
sha256 = "8527df9edae53dde4cb32bb8fbe7f671ce905e2ddf81290543f86a4b6a9304cc"

# tool_chain.toml
[source]
kind = "manual"
reference = "git:c4994d4a177d9aee9c4713bef46fad2367e0cf1e:.claude/skills/opi-eval/references/test-cases.md#case-2-tool-chain"
sha256 = "a6c03e35c51214c5ba422bec33a81b88bc71cd825c965061d82151f346886095"

# context_retention.toml
[source]
kind = "manual"
reference = "git:c4994d4a177d9aee9c4713bef46fad2367e0cf1e:.claude/skills/opi-eval/references/test-cases.md#case-3-context-retention"
sha256 = "db618972f72dc909fbf691b9d3aab04c5b3753203f4bb21679ee0096f64e8a55"
```

Add the same reviewed metadata to each manifest:

```toml

[review]
rubric_owner = "opi-maintainers"
reviewers = ["original-case-author", "local-eval-design-review"]
reviewed_at = "2026-08-11T00:00:00Z"
privacy_status = "approved"
```

Use these exact case-specific fields and assertions:

```toml
# candy.toml
schema_version = 1
id = "candy"
version = 1
suite = "local-smoke"
status = "pinned"
risk_class = "read-only"
capture_paths = []

[execution]
tool_profile = "none"
timeout_seconds = 120
default_trials = 1
minimum_pass_rate = 1.0

[task]
user_value = "Obtain the correct guaranteed draw count without external tools."
expected_behavior = "Return the minimum guaranteed draw count, 21."

[[assertions]]
id = "outcome.exit-success"
kind = "exit_code_equals"
severity = "required"
expected = 0

[[assertions]]
id = "outcome.answer-21"
kind = "final_text_regex"
severity = "required"
pattern = "(?<!\\d)21(?!\\d)"

[[assertions]]
id = "trajectory.no-tools"
kind = "tool_call_count"
severity = "required"
minimum = 0
maximum = 0
```

```toml
# tool_chain.toml
schema_version = 1
id = "tool_chain"
version = 1
suite = "local-smoke"
status = "pinned"
risk_class = "workspace-write"
capture_paths = ["result.txt"]

[execution]
tool_profile = "mutating"
timeout_seconds = 120
default_trials = 1
minimum_pass_rate = 1.0

[[fixtures.files]]
path = "test-fixture.txt"
content = """alpha
bravo
charlie
delta
echo
foxtrot
golf
hotel
india
juliet
"""

[task]
user_value = "Reliably complete a bounded file transformation."
expected_behavior = "Read the fixture, count ten lines, and write only 10 to result.txt."

[[assertions]]
id = "outcome.exit-success"
kind = "exit_code_equals"
severity = "required"
expected = 0

[[assertions]]
id = "outcome.result-exists"
kind = "file_exists"
severity = "required"
path = "result.txt"
expected = true

[[assertions]]
id = "outcome.result-text"
kind = "file_text_equals"
severity = "required"
path = "result.txt"
expected = "10"
allow_trailing_newline = true

[[assertions]]
id = "outcome.confirmation"
kind = "final_text_regex"
severity = "advisory"
pattern = "(?i)\\b(completed|done|wrote|written|created)\\b"

[[assertions]]
id = "trajectory.read-before-write"
kind = "tool_call_sequence"
severity = "required"
mode = "ordered-subsequence"
expected = ["read", "write"]

[[assertions]]
id = "trajectory.call-count"
kind = "tool_call_count"
severity = "advisory"
minimum = 2
maximum = 3

[[assertions]]
id = "trajectory.no-tool-errors"
kind = "no_tool_error"
severity = "required"
expected = true
```

```toml
# context_retention.toml
schema_version = 1
id = "context_retention"
version = 1
suite = "local-smoke"
status = "pinned"
risk_class = "read-only"
capture_paths = []

[execution]
tool_profile = "none"
timeout_seconds = 120
default_trials = 1
minimum_pass_rate = 1.0

[task]
user_value = "Answer a constrained budget question without distraction from irrelevant context."
expected_behavior = "State that 10,700 dollars remain and that this funds all 16 groups requiring 8,000 dollars."

[[assertions]]
id = "outcome.exit-success"
kind = "exit_code_equals"
severity = "required"
expected = 0

[[assertions]]
id = "outcome.remaining-budget"
kind = "final_text_regex"
severity = "required"
pattern = "10[,.]?700"

[[assertions]]
id = "outcome.group-count"
kind = "final_text_regex"
severity = "required"
pattern = "(?<!\\d)16(?!\\d)"

[[assertions]]
id = "outcome.required-budget"
kind = "final_text_regex"
severity = "required"
pattern = "8[,.]?000"

[[assertions]]
id = "outcome.sufficient"
kind = "final_text_regex"
severity = "required"
pattern = "(?i)\\b(yes|enough|sufficient)\\b"
```

Place each case-specific `[source]` table and the shared `[review]` table before
`[execution]`, and add
the moved multiline `prompt` key immediately under each `[task]` table.

- [ ] **Step 5: Run the focused tests and validate all manifests**

Run:

```text
python -m unittest .claude/skills/opi-eval/scripts/test_opi_eval.py
python .claude/skills/opi-eval/scripts/opi_eval.py validate
```

Expected: eight manifest tests pass; `validate` is still absent and the second
command fails with an argparse error. Add this initial command surface, which
later tasks extend without renaming its functions:

```python
def _cases_root() -> Path:
    return Path(__file__).resolve().parents[1] / "cases"


def cmd_validate(args: argparse.Namespace) -> int:
    selected = set(args.case or [])
    cases = [load_case(path) for path in sorted(_cases_root().glob("*.toml"))]
    registry = validate_registry(cases)
    if selected:
        registry = tuple(case for case in registry if case.id in selected)
        missing = selected - {case.id for case in registry}
        if missing:
            raise EvalError(f"unknown case ids: {sorted(missing)}")
    print(f"validated {len(registry)} cases")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate", help="validate case manifests")
    validate.add_argument("--case", action="append")
    validate.set_defaults(func=cmd_validate)
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        return int(args.func(args))
    except EvalError as error:
        print(f"opi-eval: {error}", file=sys.stderr)
        return 1
    except (OSError, UnicodeError, json.JSONDecodeError,
            tomllib.TOMLDecodeError) as error:
        print(f"opi-eval: local artifact error: {type(error).__name__}",
              file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
```

Re-run both commands; both pass.

- [ ] **Step 6: Conditional checkpoint commit**

Only with explicit commit authorization:

```text
git add .claude/skills/opi-eval/scripts/opi_eval.py
git add .claude/skills/opi-eval/scripts/test_opi_eval.py
git add .claude/skills/opi-eval/cases/candy.toml
git add .claude/skills/opi-eval/cases/tool_chain.toml
git add .claude/skills/opi-eval/cases/context_retention.toml
git commit -m "feat(opi-eval): add structured local cases"
```

### Task 2: Project NDJSON evidence and implement deterministic graders

**Files:**
- Modify: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`

- [ ] **Step 1: Add failing projection and verdict tests**

Append tests that construct an in-memory evidence object and assert the
authority rules:

```python
class GradingTests(unittest.TestCase):
    def setUp(self) -> None:
        self.case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        self.evidence = {
            "process": {"exit_code": 0},
            "assistant": {"final_text": "Completed."},
            "trajectory": {
                "tool_calls": ["read", "write"],
                "tool_completions": 2,
                "tool_errors": 0,
                "retry_count": 0,
            },
            "captured": {
                "files": {
                    "result.txt": {"text": "10\n", "sha256": "917df332"}
                }
            },
        }

    def test_tool_chain_passes_from_final_file_state(self) -> None:
        grade = opi_eval.grade_evidence(self.case, self.evidence)
        self.assertEqual(grade["verdict"], "PASS")

    def test_tool_events_cannot_hide_wrong_final_file(self) -> None:
        self.evidence["captured"]["files"]["result.txt"]["text"] = "9\n"
        grade = opi_eval.grade_evidence(self.case, self.evidence)
        self.assertEqual(grade["verdict"], "FAIL")
        self.assertEqual(grade["results"]["outcome.result-text"]["value"], 0)

    def test_missing_required_file_is_inconclusive(self) -> None:
        self.evidence.pop("captured")
        grade = opi_eval.grade_evidence(self.case, self.evidence)
        self.assertEqual(grade["verdict"], "INCONCLUSIVE")

    def test_required_failure_dominates_unknown(self) -> None:
        self.evidence["process"]["exit_code"] = 1
        self.evidence["captured"]["files"] = {}
        grade = opi_eval.grade_evidence(self.case, self.evidence)
        self.assertEqual(grade["verdict"], "FAIL")

    def test_advisory_failure_is_degraded(self) -> None:
        self.evidence["trajectory"]["tool_calls"] = ["read", "ls", "grep", "write"]
        grade = opi_eval.grade_evidence(self.case, self.evidence)
        self.assertEqual(grade["verdict"], "DEGRADED")

    def test_candy_manifest_grades_expected_answer(self) -> None:
        case = opi_eval.load_case(Path(__file__).parents[1] / "cases" / "candy.toml")
        evidence = {
            "process": {"exit_code": 0},
            "assistant": {"final_text": "The guaranteed minimum is 21."},
            "trajectory": {"tool_calls": [], "tool_completions": 0,
                           "tool_errors": 0, "retry_count": 0},
            "captured": {"files": {}},
        }
        self.assertEqual(opi_eval.grade_evidence(case, evidence)["verdict"], "PASS")

    def test_context_case_requires_all_budget_facts_and_sufficiency(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "context_retention.toml"
        )
        evidence = {
            "process": {"exit_code": 0},
            "assistant": {
                "final_text": "10,700 remains; 16 groups need 8,000, so yes, it is enough."
            },
            "trajectory": {"tool_calls": [], "tool_completions": 0,
                           "tool_errors": 0, "retry_count": 0},
            "captured": {"files": {}},
        }
        self.assertEqual(opi_eval.grade_evidence(case, evidence)["verdict"], "PASS")
        evidence["assistant"]["final_text"] = (
            "10,700 remains and 16 groups require 8,000."
        )
        self.assertEqual(opi_eval.grade_evidence(case, evidence)["verdict"], "FAIL")
```

- [ ] **Step 2: Run the grading tests and verify they fail**

Run the unittest command from Task 1.

Expected: seven errors report that `grade_evidence` does not exist.

- [ ] **Step 3: Implement evidence projection and assertion evaluation**

Add an `AssertionResult` dataclass and these functions. NDJSON projection must
accept both top-level session events and `{"type":"Agent","event":...}`
wrappers used by JSON mode.

```python
@dataclasses.dataclass(frozen=True)
class AssertionResult:
    value: int | str
    evidence: str


def _message_text(message: dict[str, Any]) -> str:
    parts = []
    for item in message.get("content", []):
        if isinstance(item, dict) and item.get("type") == "text":
            parts.append(str(item.get("text", "")))
    return "".join(parts)


def project_ndjson(text: str) -> dict[str, Any]:
    final_text: str | None = None
    tool_calls: list[str] = []
    tool_completions = 0
    tool_errors = 0
    retry_count = 0
    summary: dict[str, Any] | None = None
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            raise EvalError(f"malformed NDJSON line {line_number}: {error}") from error
        if record.get("type") == "session_summary":
            summary = record
            continue
        event = record.get("event", record)
        event_type = event.get("type")
        if event_type == "MessageEnd":
            final_text = _message_text(dict(event.get("message", {})))
        elif event_type == "ToolExecutionStart":
            tool_calls.append(str(event.get("tool_name", "")))
        elif event_type == "ToolExecutionEnd":
            tool_completions += 1
            if event.get("is_error") is True:
                tool_errors += 1
        elif event_type == "AutoRetryStart":
            retry_count += 1
    return {
        "assistant": {"final_text": final_text},
        "trajectory": {
            "tool_calls": tool_calls,
            "tool_completions": tool_completions,
            "tool_errors": tool_errors,
            "retry_count": retry_count,
        },
        "session_summary": summary,
    }


def _ordered_subsequence(expected: list[str], actual: list[str]) -> bool:
    position = 0
    for value in actual:
        if position < len(expected) and value == expected[position]:
            position += 1
    return position == len(expected)


def evaluate_assertion(
    assertion: AssertionSpec, evidence: dict[str, Any]
) -> AssertionResult:
    options = assertion.options
    process = evidence.get("process", {})
    assistant = evidence.get("assistant", {})
    trajectory = evidence.get("trajectory", {})
    captured = evidence.get("captured")
    files = None if captured is None else captured.get("files")
    kind = assertion.kind
    if kind == "exit_code_equals":
        actual = process.get("exit_code")
        if actual is None:
            return AssertionResult("unknown", "process exit code is missing")
        return AssertionResult(int(actual == options["expected"]), f"exit_code={actual}")
    if kind == "final_text_regex":
        actual = assistant.get("final_text")
        if actual is None:
            return AssertionResult("unknown", "final assistant text is missing")
        matched = re.search(str(options["pattern"]), str(actual)) is not None
        return AssertionResult(int(matched), f"pattern={options['pattern']!r}")
    if kind == "file_exists":
        if files is None:
            return AssertionResult("unknown", "capture evidence is missing")
        present = options["path"] in files
        return AssertionResult(int(present == bool(options["expected"])),
                               f"path={options['path']!r} present={present}")
    if kind == "file_text_equals":
        if files is None:
            return AssertionResult("unknown", "capture evidence is missing")
        entry = files.get(options["path"])
        if entry is None:
            return AssertionResult("unknown", f"captured file missing: {options['path']}")
        actual = str(entry.get("text", ""))
        if options.get("allow_trailing_newline") and actual.endswith("\n"):
            actual = actual[:-1]
        return AssertionResult(int(actual == str(options["expected"])),
                               f"path={options['path']!r} sha256={entry.get('sha256')}")
    if kind == "tool_call_sequence":
        if "tool_calls" not in trajectory:
            return AssertionResult("unknown", "tool call evidence is missing")
        actual = list(trajectory["tool_calls"])
        return AssertionResult(int(_ordered_subsequence(list(options["expected"]), actual)),
                               f"tool_calls={actual!r}")
    if kind == "tool_call_count":
        if "tool_calls" not in trajectory:
            return AssertionResult("unknown", "tool call evidence is missing")
        count = len(trajectory["tool_calls"])
        passed = int(options["minimum"]) <= count <= int(options["maximum"])
        return AssertionResult(int(passed), f"tool_call_count={count}")
    if kind == "no_tool_error":
        count = trajectory.get("tool_errors")
        completions = trajectory.get("tool_completions")
        starts = len(trajectory.get("tool_calls", []))
        if count is None or completions is None or completions != starts:
            return AssertionResult("unknown", "tool completion evidence is missing")
        return AssertionResult(int(count == 0), f"tool_errors={count}")
    if kind == "max_retry_count":
        count = trajectory.get("retry_count")
        if count is None:
            return AssertionResult("unknown", "retry evidence is missing")
        return AssertionResult(int(count <= int(options["maximum"])),
                               f"retry_count={count}")
    raise EvalError(f"unsupported assertion kind at grade time: {kind}")


def grade_evidence(case: CaseSpec, evidence: dict[str, Any]) -> dict[str, Any]:
    results = {item.id: evaluate_assertion(item, evidence) for item in case.assertions}
    required = [results[item.id].value for item in case.assertions
                if item.severity == "required"]
    advisory = [results[item.id].value for item in case.assertions
                if item.severity == "advisory"]
    if 0 in required:
        verdict = "FAIL"
    elif "unknown" in required:
        verdict = "INCONCLUSIVE"
    elif 0 in advisory:
        verdict = "DEGRADED"
    else:
        verdict = "PASS"
    normalized = {
        assertion_id: dataclasses.asdict(result)
        for assertion_id, result in sorted(results.items())
    }
    return {
        "schema_version": GRADE_SCHEMA_VERSION,
        "case_id": case.id,
        "case_version": case.version,
        "case_digest": case.digest,
        "verdict": verdict,
        "results": normalized,
    }
```

- [ ] **Step 4: Run the focused suite and confirm all grading tests pass**

Run the unittest command. Expected: fifteen tests pass.

- [ ] **Step 5: Conditional checkpoint commit**

Only with explicit commit authorization, stage the two script files and the
three manifests by exact path, then commit:

```text
git commit -m "feat(opi-eval): add deterministic graders"
```

### Task 3: Add bounded capture, leakage checks, and immutable bundles

**Files:**
- Modify: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`

- [ ] **Step 1: Write failing capture and sealing tests**

Add tests for allowlisted capture, path leakage, denylisted values, oversize
files, and byte-stable grade output:

```python
class CaptureTests(unittest.TestCase):
    def test_captures_only_allowlisted_text(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_text("10\n", encoding="utf-8")
            (workspace / "ignored.txt").write_text("ignore", encoding="utf-8")
            captured = opi_eval.capture_workspace(workspace, ("result.txt",), ())
            self.assertEqual(set(captured), {"result.txt"})
            self.assertEqual(captured["result.txt"]["text"], "10\n")

    def test_blocks_workspace_root_leakage(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory).resolve()
            (workspace / "result.txt").write_text(str(workspace), encoding="utf-8")
            with self.assertRaisesRegex(opi_eval.EvalError, "workspace path"):
                opi_eval.capture_workspace(workspace, ("result.txt",), ())

    def test_blocks_unrelated_absolute_path_leakage(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_text(
                "C:\\Users\\alice\\secret.txt", encoding="utf-8"
            )
            with self.assertRaisesRegex(opi_eval.EvalError, "absolute path"):
                opi_eval.capture_workspace(workspace, ("result.txt",), ())

    def test_blocks_explicit_sensitive_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_text("canary-secret", encoding="utf-8")
            with self.assertRaisesRegex(opi_eval.EvalError, "sensitive value"):
                opi_eval.capture_workspace(
                    workspace, ("result.txt",), ("canary-secret",)
                )

    def test_blocks_oversize_capture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_bytes(b"x" * (opi_eval.MAX_CAPTURE_BYTES + 1))
            with self.assertRaisesRegex(opi_eval.EvalError, "size limit"):
                opi_eval.capture_workspace(workspace, ("result.txt",), ())

    def test_blocks_non_utf8_capture(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_bytes(b"\xff\xfe")
            with self.assertRaisesRegex(opi_eval.EvalError, "not UTF-8"):
                opi_eval.capture_workspace(workspace, ("result.txt",), ())

    def test_blocks_known_secret_pattern(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            (workspace / "result.txt").write_text(
                "ghp_abcdefghijklmnopqrstuvwxyz", encoding="utf-8"
            )
            with self.assertRaisesRegex(opi_eval.EvalError, "secret pattern"):
                opi_eval.capture_workspace(workspace, ("result.txt",), ())

    def test_normalized_grade_is_byte_stable(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "candy.toml"
        )
        evidence = {
            "process": {"exit_code": 0},
            "assistant": {"final_text": "21"},
            "trajectory": {
                "tool_calls": [], "tool_completions": 0,
                "tool_errors": 0, "retry_count": 0,
            },
            "captured": {"files": {}},
        }
        first = opi_eval.normalized_json(opi_eval.grade_evidence(case, evidence))
        second = opi_eval.normalized_json(opi_eval.grade_evidence(case, evidence))
        self.assertEqual(first, second)

    def test_seal_failure_leaves_no_partial_bundle(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            workspace = root / "workspace"
            staging = root / "staging"
            bundle = root / "bundle"
            workspace.mkdir()
            staging.mkdir()
            (workspace / "result.txt").write_text("10\n", encoding="utf-8")
            (staging / "output.ndjson").write_text(
                json.dumps({"type": "diagnostic", "path": str(workspace.resolve())}),
                encoding="utf-8",
            )
            with self.assertRaisesRegex(opi_eval.EvalError, "workspace path"):
                opi_eval.seal_trial(
                    staging=staging,
                    workspace=workspace,
                    bundle_root=bundle,
                    trial_id="trial-001",
                    case=case,
                    exit_code=0,
                    duration_seconds=1.0,
                    stderr="",
                    sensitive_values=(),
                )
            self.assertFalse(bundle.exists())
```

- [ ] **Step 2: Run tests and verify the capture tests fail**

Expected: nine errors for missing `capture_workspace`, `normalized_json`, or
`seal_trial`.

- [ ] **Step 3: Implement capture and atomic JSON helpers**

Add exact helpers with no recursive workspace copy:

```python
KNOWN_SECRET_PATTERNS = (
    re.compile(r"\bghp_[A-Za-z0-9]{20,}\b"),
    re.compile(r"\bsk-[A-Za-z0-9_-]{20,}\b"),
    re.compile(r"(?i)\bBearer\s+[A-Za-z0-9._~-]{16,}\b"),
    re.compile(r"https?://[^\s/:]+:[^\s/@]+@"),
)
ABSOLUTE_PATH_PATTERNS = (
    re.compile(r"(?<![A-Za-z0-9])[A-Za-z]:[\\/][^\s\"'<>|]+"),
    re.compile(r"(?<![A-Za-z0-9:])/(?:home|Users|tmp|var/tmp)/[^\s\"'<>]+"),
)


def normalized_json(value: Any) -> bytes:
    return _canonical_bytes(value)


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _scan_text(text: str, workspace: Path, sensitive_values: tuple[str, ...]) -> None:
    root = str(workspace.resolve())
    variants = {root, root.replace("\\", "/"), json.dumps(root)[1:-1]}
    if any(value and value in text for value in variants):
        raise EvalError("captured text contains workspace path")
    for value in sensitive_values:
        if value and value in text:
            raise EvalError("captured text contains configured sensitive value")
    for pattern in KNOWN_SECRET_PATTERNS:
        if pattern.search(text):
            raise EvalError(f"captured text matches secret pattern: {pattern.pattern}")
    for pattern in ABSOLUTE_PATH_PATTERNS:
        if pattern.search(text):
            raise EvalError("captured text contains an absolute path")


def capture_workspace(
    workspace: Path,
    capture_paths: tuple[str, ...],
    sensitive_values: tuple[str, ...],
) -> dict[str, dict[str, Any]]:
    root = workspace.resolve()
    captured: dict[str, dict[str, Any]] = {}
    for relative in capture_paths:
        normalized = _safe_relative_path(relative)
        lexical = root / Path(normalized)
        cursor = root
        for part in Path(normalized).parts:
            cursor /= part
            if cursor.is_symlink():
                raise EvalError(f"capture path traverses a symlink: {normalized}")
        source = lexical.resolve()
        if root not in source.parents:
            raise EvalError(f"capture path escaped workspace: {normalized}")
        if not source.exists():
            continue
        if source.is_symlink() or not source.is_file():
            raise EvalError(f"capture path is not a regular file: {normalized}")
        data = source.read_bytes()
        if len(data) > MAX_CAPTURE_BYTES:
            raise EvalError(f"capture exceeds size limit: {normalized}")
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError as error:
            raise EvalError(f"capture is not UTF-8 text: {normalized}") from error
        _scan_text(text, root, sensitive_values)
        captured[normalized] = {
            "size": len(data),
            "sha256": _sha256_bytes(data),
            "text": text,
        }
    return captured


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f"{path.name}.tmp-{os.getpid()}-{uuid.uuid4().hex}")
    temporary.write_bytes(normalized_json(value))
    os.replace(temporary, path)
```

- [ ] **Step 4: Add `seal_trial` without exposing unsafe staging evidence**

Add this function. It scans stdout, stderr, and captures before creating the
bundle path; any write failure cleans up only the validated trial directory.

```python
TRIAL_ID_RE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]*$")


def seal_trial(
    staging: Path,
    workspace: Path,
    bundle_root: Path,
    trial_id: str,
    case: CaseSpec,
    exit_code: int,
    duration_seconds: float,
    stderr: str,
    sensitive_values: tuple[str, ...],
) -> dict[str, Any]:
    _require(bool(TRIAL_ID_RE.fullmatch(trial_id)), "invalid trial id")
    output_path = staging / "output.ndjson"
    output_bytes = output_path.read_bytes()
    _require(len(output_bytes) <= MAX_NDJSON_BYTES, "NDJSON exceeds size limit")
    _require(len(stderr.encode("utf-8")) <= MAX_STDERR_BYTES,
             "stderr exceeds size limit")
    try:
        output_text = output_bytes.decode("utf-8")
    except UnicodeDecodeError as error:
        raise EvalError("NDJSON is not UTF-8") from error
    _scan_text(output_text, workspace, sensitive_values)
    _scan_text(stderr, workspace, sensitive_values)
    projected = project_ndjson(output_text)
    captured_files = capture_workspace(
        workspace, case.capture_paths, sensitive_values
    )
    evidence = {
        **projected,
        "process": {"exit_code": exit_code},
        "captured": {"files": captured_files},
    }
    root = bundle_root.resolve()
    trials_root = (root / "trials").resolve()
    bundle_trial = (trials_root / trial_id).resolve()
    _require(bundle_trial.parent == trials_root, "trial path escaped run bundle")
    _require(not bundle_trial.exists(), f"trial bundle already exists: {trial_id}")
    capture_manifest = {
        "schema_version": 1,
        "files": {
            path: {
                "size": entry["size"],
                "sha256": entry["sha256"],
                "media_type": "text/plain; charset=utf-8",
            }
            for path, entry in sorted(captured_files.items())
        },
    }
    trial_record = {
        "schema_version": 1,
        "trial_id": trial_id,
        "case_id": case.id,
        "case_version": case.version,
        "case_digest": case.digest,
        "exit_code": exit_code,
        "duration_seconds": duration_seconds,
        "stderr": stderr,
    }
    try:
        (bundle_trial / "captured").mkdir(parents=True)
        for path, entry in sorted(captured_files.items()):
            destination = bundle_trial / "captured" / Path(path)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(entry["text"].encode("utf-8"))
        atomic_json(bundle_trial / "capture-manifest.json", capture_manifest)
        atomic_json(bundle_trial / "trial.json", trial_record)
        (bundle_trial / "output.ndjson").write_bytes(output_bytes)
    except Exception:
        if bundle_trial.exists() and bundle_trial.parent == trials_root:
            shutil.rmtree(bundle_trial)
        raise
    return {
        "evidence": evidence,
        "trial_dir": bundle_trial,
        "artifact_digests": {
            name: _sha256_bytes((bundle_trial / name).read_bytes())
            for name in ("output.ndjson", "trial.json", "capture-manifest.json")
        },
    }
```

- [ ] **Step 5: Run tests and verify capture behavior**

Run the unittest command. Expected: twenty-four tests pass.

- [ ] **Step 6: Conditional checkpoint commit**

Only with explicit authorization, stage the two script files and commit:

```text
git commit -m "feat(opi-eval): seal bounded run evidence"
```

### Task 4: Implement isolated trial execution and Cargo-cache-safe builds

**Files:**
- Modify: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`

- [ ] **Step 1: Write a failing fake-executable runner test**

The test creates a Python fake that accepts Opi-shaped arguments, writes
`result.txt`, and emits valid NDJSON. It calls the internal runner with
`[sys.executable, fake_path]`, so it works on Windows and Unix.

```python
class RunnerTests(unittest.TestCase):
    def test_run_trial_seals_workspace_outcome(self) -> None:
        fake_source = '''
import json
from pathlib import Path
Path("result.txt").write_text("10\\n", encoding="utf-8")
events = [
    {"type":"Agent","event":{"type":"ToolExecutionStart","tool_call_id":"r1","tool_name":"read","args":{"path":"test-fixture.txt"}}},
    {"type":"Agent","event":{"type":"ToolExecutionEnd","tool_call_id":"r1","tool_name":"read","result":{},"is_error":False,"truncated":False}},
    {"type":"Agent","event":{"type":"ToolExecutionStart","tool_call_id":"w1","tool_name":"write","args":{"path":"result.txt","content":"10"}}},
    {"type":"Agent","event":{"type":"ToolExecutionEnd","tool_call_id":"w1","tool_name":"write","result":{},"is_error":False,"truncated":False}},
    {"type":"Agent","event":{"type":"MessageEnd","message":{"content":[{"type":"text","text":"Completed."}]}}},
    {"type":"session_summary","session_id":"fixture","model":"mock:model","turns":1,"provider_turns":2,"tokens":{"input":1,"output":1,"cache_read":0,"cache_write":0}},
]
for event in events:
    print(json.dumps(event, separators=(",", ":")))
'''
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fake = root / "fake_opi.py"
            fake.write_text(fake_source, encoding="utf-8")
            bundle = root / "bundle"
            result = opi_eval.run_trial(
                case=case,
                binary_command=[sys.executable, str(fake)],
                model="mock:model",
                trial_id="trial-001",
                bundle_root=bundle,
                sensitive_values=(),
            )
            self.assertEqual(result["grade"]["verdict"], "PASS")
            self.assertEqual(
                (bundle / "trials" / "trial-001" / "captured" / "result.txt")
                .read_text(encoding="utf-8"),
                "10\n",
            )

    def test_timeout_is_recorded_as_error_without_retry(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "candy.toml"
        )
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "bundle"
            timeout = subprocess.TimeoutExpired(["fake-opi"], 1, stderr="timed out")
            with mock.patch("subprocess.run", side_effect=timeout) as run:
                result = opi_eval.run_trial(
                    case=case,
                    binary_command=["fake-opi"],
                    model="mock:model",
                    trial_id="trial-001",
                    bundle_root=bundle,
                    sensitive_values=(),
                )
            self.assertEqual(run.call_count, 1)
            self.assertEqual(result["grade"]["verdict"], "ERROR")
```

- [ ] **Step 2: Run the test and verify it fails for missing `run_trial`**

Run the focused unittest command. Expected: two new errors.

- [ ] **Step 3: Implement fixture creation and one-trial execution**

Add these helpers and `run_trial`. The timeout path writes a bounded error
artifact and never retries. Normalized grade content contains no generation
time.

```python
def _grader_digest(case: CaseSpec) -> str:
    script_digest = _sha256_bytes(Path(__file__).read_bytes())
    return _sha256_bytes(_canonical_bytes({
        "grade_schema_version": GRADE_SCHEMA_VERSION,
        "script_sha256": script_digest,
        "case_digest": case.digest,
    }))


def _write_fixtures(workspace: Path, case: CaseSpec) -> None:
    root = workspace.resolve()
    for fixture in case.fixtures:
        destination = (root / Path(fixture["path"])).resolve()
        if root not in destination.parents:
            raise EvalError(f"fixture escaped workspace: {fixture['path']}")
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(fixture["content"], encoding="utf-8", newline="")


def _process_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        try:
            return value.decode("utf-8")
        except UnicodeDecodeError as error:
            raise EvalError("process output is not UTF-8") from error
    return value


def _write_error_trial(
    bundle_root: Path,
    trial_id: str,
    case: CaseSpec,
    kind: str,
    message: str,
    stderr: str,
    workspace: Path,
    sensitive_values: tuple[str, ...],
) -> dict[str, Any]:
    _require(bool(TRIAL_ID_RE.fullmatch(trial_id)), "invalid trial id")
    _require(len(stderr.encode("utf-8")) <= MAX_STDERR_BYTES,
             "stderr exceeds size limit")
    _scan_text(stderr, workspace, sensitive_values)
    root = bundle_root.resolve()
    trials_root = (root / "trials").resolve()
    trial_dir = (trials_root / trial_id).resolve()
    _require(trial_dir.parent == trials_root, "trial path escaped run bundle")
    _require(not trial_dir.exists(), f"trial bundle already exists: {trial_id}")
    trial = {
        "schema_version": 1,
        "trial_id": trial_id,
        "case_id": case.id,
        "case_version": case.version,
        "case_digest": case.digest,
        "status": "ERROR",
        "error": {"kind": kind, "message": message},
        "stderr": stderr,
    }
    grade = {
        "schema_version": GRADE_SCHEMA_VERSION,
        "case_id": case.id,
        "case_version": case.version,
        "case_digest": case.digest,
        "verdict": "ERROR",
        "results": {},
        "error": {"kind": kind, "message": message},
    }
    grade_name = f"grade-{_grader_digest(case)}.json"
    try:
        trial_dir.mkdir(parents=True)
        atomic_json(trial_dir / "trial.json", trial)
        atomic_json(trial_dir / grade_name, grade)
    except Exception:
        if trial_dir.exists() and trial_dir.parent == trials_root:
            shutil.rmtree(trial_dir)
        raise
    return {
        "grade": grade,
        "trial_dir": trial_dir,
        "artifact_digests": {
            "trial.json": _sha256_bytes((trial_dir / "trial.json").read_bytes()),
            grade_name: _sha256_bytes((trial_dir / grade_name).read_bytes()),
        },
    }


def run_trial(
    case: CaseSpec,
    binary_command: list[str],
    model: str | None,
    trial_id: str,
    bundle_root: Path,
    sensitive_values: tuple[str, ...],
) -> dict[str, Any]:
    with tempfile.TemporaryDirectory(prefix="opi-eval-") as directory:
        temporary = Path(directory)
        workspace = temporary / "workspace"
        staging = temporary / "staging"
        sessions = workspace / ".sessions"
        workspace.mkdir()
        staging.mkdir()
        sessions.mkdir()
        _write_fixtures(workspace, case)
        command = [*binary_command, "--json"]
        if model is not None:
            command.extend(["--model", model])
        if case.execution["tool_profile"] == "none":
            command.append("--no-builtin-tools")
        else:
            command.append("--allow-mutating")
        command.append(case.prompt)
        environment = os.environ.copy()
        environment["OPI_SESSIONS_DIR"] = str(sessions)
        started = time.monotonic()
        try:
            completed = subprocess.run(
                command,
                cwd=workspace,
                env=environment,
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="strict",
                timeout=int(case.execution["timeout_seconds"]),
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            return _write_error_trial(
                bundle_root=bundle_root,
                trial_id=trial_id,
                case=case,
                kind="timeout",
                message="trial exceeded its pre-registered timeout",
                stderr=_process_text(error.stderr),
                workspace=workspace,
                sensitive_values=sensitive_values,
            )
        duration = time.monotonic() - started
        (staging / "output.ndjson").write_text(
            completed.stdout, encoding="utf-8", newline=""
        )
        sealed = seal_trial(
            staging=staging,
            workspace=workspace,
            bundle_root=bundle_root,
            trial_id=trial_id,
            case=case,
            exit_code=completed.returncode,
            duration_seconds=duration,
            stderr=completed.stderr,
            sensitive_values=sensitive_values,
        )
        grade = grade_evidence(case, sealed["evidence"])
        grade_name = f"grade-{_grader_digest(case)}.json"
        grade_path = sealed["trial_dir"] / grade_name
        atomic_json(grade_path, grade)
        sealed["artifact_digests"][grade_name] = _sha256_bytes(
            grade_path.read_bytes()
        )
        return {**sealed, "grade": grade}
```

- [ ] **Step 4: Implement Cargo target resolution, lease, and fresh build**

Add `build_release_binary(workspace_root)` with this exact policy:

```python
def build_release_binary(workspace_root: Path) -> tuple[Path, Path | None]:
    explicit = os.environ.get("CARGO_TARGET_DIR")
    managed_target: Path | None = None
    if explicit:
        target = Path(explicit).expanduser().resolve()
    else:
        resolved = subprocess.run(
            [sys.executable, "scripts/opi-cargo-cache.py", "resolve"],
            cwd=workspace_root, check=True, capture_output=True, text=True,
            encoding="utf-8",
        )
        target = Path(resolved.stdout.strip()).resolve()
        managed_target = target
        subprocess.run(
            [sys.executable, "scripts/opi-cargo-cache.py", "lease", "start",
             "--target", str(target), "--pid", str(os.getpid())],
            cwd=workspace_root, check=True,
        )
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(target)
    try:
        subprocess.run(
            ["cargo", "build", "--release", "-p", "opi-coding-agent"],
            cwd=workspace_root, env=environment, check=True,
        )
    finally:
        if managed_target is not None:
            subprocess.run(
                [sys.executable, "scripts/opi-cargo-cache.py", "lease", "end",
                 "--target", str(managed_target), "--pid", str(os.getpid())],
                cwd=workspace_root, check=True,
            )
    binary = target / "release" / ("opi.exe" if os.name == "nt" else "opi")
    if not binary.is_file():
        raise EvalError(f"fresh release binary is missing: {binary}")
    return binary, managed_target
```

Test this function with `unittest.mock.patch("subprocess.run")`; assert the
lease-end call occurs in `finally` after a simulated Cargo failure. Never run
Cargo from the unit test.

```python
    def test_managed_cargo_lease_ends_after_build_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory)
            target = workspace / "managed-target"

            def fake_run(command: list[str], **kwargs: object) -> subprocess.CompletedProcess:
                if "resolve" in command:
                    return subprocess.CompletedProcess(command, 0, stdout=str(target))
                if command[0] == "cargo":
                    raise subprocess.CalledProcessError(1, command)
                return subprocess.CompletedProcess(command, 0)

            with mock.patch.dict(os.environ, {"CARGO_TARGET_DIR": ""}), \
                 mock.patch("subprocess.run", side_effect=fake_run) as run:
                with self.assertRaises(subprocess.CalledProcessError):
                    opi_eval.build_release_binary(workspace)
            commands = [call.args[0] for call in run.call_args_list]
            self.assertTrue(any(command[2:4] == ["lease", "end"]
                                for command in commands))
```

- [ ] **Step 5: Add the `run` command and resolved run manifest**

The `run` command selects pinned cases by default, builds once, creates
`target/opi-eval/20260811T120000Z-a1b2c3d4/manifest.json` shape, using the real
UTC time and fresh random suffix, and executes each
trial. The manifest records schema version, case digests, Git commit,
dirty-worktree boolean, binary SHA-256, requested model or `null`, tool
profiles, platform, architecture, and start time.

Extend the parser with repeatable `--case`, optional `--model`, optional
positive `--trials`, and optional `--sensitive-values-file`. The sensitive
values file is UTF-8 with one non-empty literal per line. Pass the resulting
tuple to capture/report scans; record only the file's SHA-256 in the run
manifest, never its path or contents. Reject a missing, non-regular, oversized,
or non-UTF-8 denylist before build or provider use.

After each trial, read the actual model from `session_summary.model`. Mark
`control_coverage` as `partial` when the caller omitted `--model`; such a run
cannot become an efficiency baseline. Never silently load a prior run bundle.

Add the lifecycle-selection test first:

```python
    def test_default_run_selects_only_pinned_and_rejects_retired(self) -> None:
        pinned = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "candy.toml"
        )
        candidate = dataclasses.replace(
            pinned, id="candidate_case", status="candidate", review=None
        )
        retired = dataclasses.replace(
            pinned,
            id="retired_case",
            status="retired",
            retirement={
                "reason": "superseded",
                "retired_at": "2026-08-11T00:00:00Z",
                "replacement_case": "candy",
            },
        )
        registry = (candidate, pinned, retired)
        self.assertEqual(
            [case.id for case in opi_eval.select_run_cases(registry, ())],
            ["candy"],
        )
        with self.assertRaisesRegex(opi_eval.EvalError, "retired"):
            opi_eval.select_run_cases(registry, ("retired_case",))
```

Then add the exact command implementation and register `cmd_run` in
`build_parser`:

```python
def _workspace_root() -> Path:
    return Path(__file__).resolve().parents[4]


def _load_registry() -> tuple[CaseSpec, ...]:
    return validate_registry(
        load_case(path) for path in sorted(_cases_root().glob("*.toml"))
    )


def select_run_cases(
    registry: tuple[CaseSpec, ...], requested_ids: tuple[str, ...]
) -> tuple[CaseSpec, ...]:
    by_id = {case.id: case for case in registry}
    if requested_ids:
        if len(requested_ids) != len(set(requested_ids)):
            raise EvalError("run case ids must be unique")
        missing = set(requested_ids) - set(by_id)
        if missing:
            raise EvalError(f"unknown case ids: {sorted(missing)}")
        selected = tuple(by_id[case_id] for case_id in requested_ids)
        retired = [case.id for case in selected if case.status == "retired"]
        if retired:
            raise EvalError(f"retired cases cannot run: {retired}")
        return selected
    return tuple(case for case in registry if case.status == "pinned")


def _positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be positive")
    return parsed


def _load_sensitive_values(path: Path | None) -> tuple[tuple[str, ...], str | None]:
    if path is None:
        return (), None
    if not path.is_file() or path.is_symlink():
        raise EvalError("sensitive-values file must be a regular file")
    data = path.read_bytes()
    if len(data) > MAX_CAPTURE_BYTES:
        raise EvalError("sensitive-values file exceeds size limit")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise EvalError("sensitive-values file is not UTF-8") from error
    values = tuple(line for line in (item.strip() for item in text.splitlines())
                   if line)
    return values, _sha256_bytes(data)


def _git_state(workspace_root: Path) -> tuple[str, bool]:
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=workspace_root, check=True,
        capture_output=True, text=True, encoding="utf-8",
    ).stdout.strip()
    status = subprocess.run(
        ["git", "status", "--porcelain"], cwd=workspace_root, check=True,
        capture_output=True, text=True, encoding="utf-8",
    ).stdout
    return commit, bool(status)


def _control_fingerprint(
    binary_sha256: str,
    requested_model: str | None,
    actual_models: tuple[str, ...],
    cases: tuple[CaseSpec, ...],
) -> str:
    return _sha256_bytes(_canonical_bytes({
        "binary_sha256": binary_sha256,
        "requested_model": requested_model,
        "actual_models": actual_models,
        "cases": {
            case.id: {
                "digest": case.digest,
                "tool_profile": case.execution["tool_profile"],
                "timeout_seconds": case.execution["timeout_seconds"],
            }
            for case in cases
        },
        "system": platform.system(),
        "machine": platform.machine(),
    }))


def cmd_run(args: argparse.Namespace) -> int:
    workspace_root = _workspace_root()
    registry = _load_registry()
    selected = select_run_cases(registry, tuple(args.case or ()))
    if not selected:
        raise EvalError("no runnable cases selected")
    sensitive_values, sensitive_digest = _load_sensitive_values(
        args.sensitive_values_file
    )
    try:
        binary, _managed_target = build_release_binary(workspace_root)
        commit, dirty = _git_state(workspace_root)
    except subprocess.CalledProcessError as error:
        raise EvalError(f"local build or Git command failed: {error.cmd}") from error
    binary_digest = _sha256_bytes(binary.read_bytes())
    started_at = datetime.now(UTC)
    run_id = f"{started_at:%Y%m%dT%H%M%SZ}-{uuid.uuid4().hex[:8]}"
    bundle_root = workspace_root / "target" / "opi-eval" / run_id
    case_artifacts: dict[str, tuple[bytes, str]] = {}
    for case in selected:
        data = case.path.read_bytes()
        if len(data) > MAX_CAPTURE_BYTES:
            raise EvalError(f"case manifest exceeds size limit: {case.id}")
        try:
            case_text = data.decode("utf-8")
        except UnicodeDecodeError as error:
            raise EvalError(f"case manifest is not UTF-8: {case.id}") from error
        _scan_text(case_text, workspace_root, sensitive_values)
        case_artifacts[case.id] = (data, _sha256_bytes(data))
    bundle_root.mkdir(parents=True)
    (bundle_root / "cases").mkdir()
    for case_id, (data, _digest) in sorted(case_artifacts.items()):
        (bundle_root / "cases" / f"{case_id}.toml").write_bytes(data)
    manifest: dict[str, Any] = {
        "schema_version": RUN_SCHEMA_VERSION,
        "run_id": run_id,
        "status": "running",
        "started_at": started_at.isoformat().replace("+00:00", "Z"),
        "commit": commit,
        "dirty": dirty,
        "binary": {"name": binary.name, "sha256": binary_digest},
        "requested_model": args.model,
        "actual_models": [],
        "control_coverage": "partial",
        "control_fingerprint": None,
        "environment_class": {
            "system": platform.system(), "machine": platform.machine()
        },
        "sensitive_values_sha256": sensitive_digest,
        "cases": {
            case.id: {
                "version": case.version,
                "digest": case.digest,
                "artifact": f"cases/{case.id}.toml",
                "artifact_sha256": case_artifacts[case.id][1],
                "user_value": case.user_value,
                "expected_behavior": case.expected_behavior,
                "minimum_pass_rate": case.execution["minimum_pass_rate"],
            }
            for case in selected
        },
        "trials": [],
    }
    atomic_json(bundle_root / "manifest.json", manifest)
    grades: list[str] = []
    actual_models: set[str] = set()
    ordinal = 0
    for case in selected:
        trial_count = args.trials or int(case.execution["default_trials"])
        for case_trial in range(1, trial_count + 1):
            ordinal += 1
            trial_id = f"{case.id}-trial-{case_trial:03d}"
            try:
                result = run_trial(
                    case=case,
                    binary_command=[str(binary)],
                    model=args.model,
                    trial_id=trial_id,
                    bundle_root=bundle_root,
                    sensitive_values=sensitive_values,
                )
                grade = result["grade"]
                summary = result.get("evidence", {}).get("session_summary")
                if isinstance(summary, dict) and summary.get("model"):
                    actual_models.add(str(summary["model"]))
                trial_record = {
                    "ordinal": ordinal,
                    "trial_id": trial_id,
                    "case_id": case.id,
                    "case_trial": case_trial,
                    "verdict": grade["verdict"],
                    "artifact_digests": result["artifact_digests"],
                }
            except EvalError as error:
                grade = {"verdict": "ERROR"}
                trial_record = {
                    "ordinal": ordinal,
                    "trial_id": trial_id,
                    "case_id": case.id,
                    "case_trial": case_trial,
                    "verdict": "ERROR",
                    "error": str(error),
                    "artifact_digests": {},
                }
            except (OSError, UnicodeError, subprocess.SubprocessError) as error:
                grade = {"verdict": "ERROR"}
                trial_record = {
                    "ordinal": ordinal,
                    "trial_id": trial_id,
                    "case_id": case.id,
                    "case_trial": case_trial,
                    "verdict": "ERROR",
                    "error": f"trial infrastructure failure: {type(error).__name__}",
                    "artifact_digests": {},
                }
            grades.append(str(grade["verdict"]))
            manifest["trials"].append(trial_record)
            atomic_json(bundle_root / "manifest.json", manifest)
    resolved_models = tuple(sorted(actual_models))
    manifest["actual_models"] = list(resolved_models)
    manifest["control_coverage"] = (
        "complete"
        if args.model is not None and resolved_models == (args.model,)
        else "partial"
    )
    manifest["control_fingerprint"] = _control_fingerprint(
        binary_digest, args.model, resolved_models, selected
    )
    manifest["status"] = "sealed"
    manifest["completed_at"] = datetime.now(UTC).isoformat().replace("+00:00", "Z")
    atomic_json(bundle_root / "manifest.json", manifest)
    print(bundle_root.relative_to(workspace_root).as_posix())
    return int(any(value in {"FAIL", "INCONCLUSIVE", "ERROR"} for value in grades))
```

Extend `build_parser()` with:

```python
    run = commands.add_parser("run", help="run fresh isolated local trials")
    run.add_argument("--case", action="append")
    run.add_argument("--model")
    run.add_argument("--trials", type=_positive_int)
    run.add_argument("--sensitive-values-file", type=Path)
    run.set_defaults(func=cmd_run)
```

- [ ] **Step 6: Run all fake-runner tests**

Run the unittest command. Expected: all tests pass with no Cargo invocation and
no network access.

- [ ] **Step 7: Conditional checkpoint commit**

Only with explicit authorization:

```text
git add .claude/skills/opi-eval/scripts/opi_eval.py
git add .claude/skills/opi-eval/scripts/test_opi_eval.py
git commit -m "feat(opi-eval): run isolated local trials"
```

### Task 5: Add machine-label validation and calibration authority

**Files:**
- Modify: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`
- Create: `.claude/skills/opi-eval/calibration/README.md`

- [ ] **Step 1: Write failing calibration tests**

Add tests for exact agreement, threshold failure, and fingerprint invalidation:

```python
class CalibrationTests(unittest.TestCase):
    def test_uncalibrated_machine_labels_are_diagnostic(self) -> None:
        authority = opi_eval.calibration_authority(None, "fingerprint-a")
        self.assertEqual(authority, "diagnostic")

    def test_matching_calibration_can_be_authoritative(self) -> None:
        calibration = {
            "schema_version": 1,
            "evaluator_fingerprint": "fingerprint-a",
            "sample_count": 20,
            "human_human_exact": 0.95,
            "human_machine_exact": 0.90,
            "unknown_rate": 0.05,
            "policy": {
                "evaluator_fingerprint": "fingerprint-a",
                "minimum_samples": 20,
                "minimum_human_human_exact": 0.90,
                "minimum_human_machine_exact": 0.90,
                "maximum_unknown_rate": 0.10,
            },
        }
        self.assertEqual(
            opi_eval.calibration_authority(calibration, "fingerprint-a"),
            "calibrated",
        )

    def test_fingerprint_change_invalidates_calibration(self) -> None:
        calibration = {
            "schema_version": 1,
            "evaluator_fingerprint": "fingerprint-a",
            "sample_count": 20,
            "human_human_exact": 1.0,
            "human_machine_exact": 1.0,
            "unknown_rate": 0.0,
            "policy": {
                "evaluator_fingerprint": "fingerprint-a",
                "minimum_samples": 1,
                "minimum_human_human_exact": 0.0,
                "minimum_human_machine_exact": 0.0,
                "maximum_unknown_rate": 1.0,
            },
        }
        self.assertEqual(
            opi_eval.calibration_authority(calibration, "fingerprint-b"),
            "diagnostic",
        )

    def test_exact_agreement_counts_unknown_as_a_label(self) -> None:
        self.assertEqual(
            opi_eval.exact_agreement([1, 0, "unknown"], [1, 1, "unknown"]),
            2 / 3,
        )
```

- [ ] **Step 2: Run tests and verify the four new failures**

Expected: missing `calibration_authority` and `exact_agreement`.

- [ ] **Step 3: Implement calibration schemas and agreement calculation**

Define machine labels as JSON objects with `schema_version`, evaluator identity,
`evaluator_fingerprint`, evidence digest, and a map from rubric ID to
`1 | 0 | "unknown"`. Reject extra rubric IDs and missing required keys.

Define gold records with evidence digest, two independent reviewer labels, and
one adjudicated label. `calibrate` computes:

```python
def _is_label_value(value: Any) -> bool:
    return not isinstance(value, bool) and (
        value == 0 or value == 1 or value == "unknown"
    )


def validate_machine_labels(
    raw: dict[str, Any],
    rubrics: tuple[SubjectiveRubric, ...],
    evidence_digest: str,
) -> dict[str, Any]:
    _require(raw.get("schema_version") == 1,
             "machine-label schema_version must be 1")
    evaluator = raw.get("evaluator")
    _require(isinstance(evaluator, dict)
             and bool(evaluator.get("provider"))
             and bool(evaluator.get("model")),
             "machine labels require evaluator provider and model")
    _require(bool(SHA256_RE.fullmatch(
                 str(raw.get("evaluator_fingerprint", "")))),
             "invalid evaluator fingerprint")
    _require(raw.get("evidence_digest") == evidence_digest,
             "machine-label evidence digest mismatch")
    labels = raw.get("labels")
    _require(isinstance(labels, dict), "machine labels must be an object")
    expected_ids = {rubric.id for rubric in rubrics}
    _require(set(labels) == expected_ids,
             "machine labels must contain exactly the supplied rubric ids")
    for rubric_id, label in labels.items():
        _require(isinstance(label, dict), f"label must be an object: {rubric_id}")
        _require(set(label) == {"value", "evidence"},
                 f"label keys are invalid: {rubric_id}")
        _require(_is_label_value(label["value"]),
                 f"label value is invalid: {rubric_id}")
        _require(isinstance(label["evidence"], str)
                 and len(label["evidence"].encode("utf-8")) <= MAX_CAPTURE_BYTES,
                 f"label evidence is invalid: {rubric_id}")
    return raw


def exact_agreement(left: list[int | str], right: list[int | str]) -> float:
    if len(left) != len(right) or not left:
        raise EvalError("agreement inputs must have equal non-zero length")
    return sum(a == b for a, b in zip(left, right, strict=True)) / len(left)


def calibration_authority(
    calibration: dict[str, Any] | None,
    evaluator_fingerprint: str,
) -> str:
    if calibration is None:
        return "diagnostic"
    if calibration.get("evaluator_fingerprint") != evaluator_fingerprint:
        return "diagnostic"
    policy = calibration["policy"]
    if policy.get("evaluator_fingerprint") != evaluator_fingerprint:
        return "diagnostic"
    passed = (
        calibration["sample_count"] >= policy["minimum_samples"]
        and calibration["human_human_exact"]
            >= policy["minimum_human_human_exact"]
        and calibration["human_machine_exact"]
            >= policy["minimum_human_machine_exact"]
        and calibration["unknown_rate"] <= policy["maximum_unknown_rate"]
    )
    return "calibrated" if passed else "diagnostic"


def _cohen_kappa(
    left: list[int | str], right: list[int | str]
) -> tuple[float | None, str | None]:
    classes = sorted(set(left) | set(right), key=str)
    if len(classes) < 2:
        return None, "fewer than two observed classes"
    observed = exact_agreement(left, right)
    total = len(left)
    expected = sum(
        (left.count(value) / total) * (right.count(value) / total)
        for value in classes
    )
    if expected >= 1.0:
        return None, "expected agreement is one"
    return (observed - expected) / (1.0 - expected), None


def compute_calibration(
    gold: dict[str, Any], machine: dict[str, Any]
) -> dict[str, Any]:
    _require(gold.get("schema_version") == 1, "gold schema_version must be 1")
    _require(machine.get("schema_version") == 1,
             "machine batch schema_version must be 1")
    fingerprint = str(machine.get("evaluator_fingerprint", ""))
    _require(bool(SHA256_RE.fullmatch(fingerprint)),
             "invalid evaluator fingerprint")
    policy = dict(gold.get("policy", {}))
    _require(bool(SHA256_RE.fullmatch(
                 str(policy.get("evaluator_fingerprint", "")))),
             "gold policy requires evaluator_fingerprint")
    machine_index: dict[tuple[str, str], int | str] = {}
    for record in machine.get("records", []):
        evidence_digest = str(record.get("evidence_digest", ""))
        _require(bool(SHA256_RE.fullmatch(evidence_digest)),
                 "invalid machine evidence digest")
        labels = record.get("labels", {})
        _require(isinstance(labels, dict), "machine record labels must be an object")
        for rubric_id, label in labels.items():
            value = label.get("value") if isinstance(label, dict) else None
            _require(_is_label_value(value), "invalid machine calibration label")
            key = (evidence_digest, str(rubric_id))
            _require(key not in machine_index, "duplicate machine calibration label")
            machine_index[key] = value
    reviewer_left: list[int | str] = []
    reviewer_right: list[int | str] = []
    adjudicated: list[int | str] = []
    machine_values: list[int | str] = []
    expected_keys: set[tuple[str, str]] = set()
    for record in gold.get("records", []):
        evidence_digest = str(record.get("evidence_digest", ""))
        rubric_id = str(record.get("rubric_id", ""))
        _require(bool(SHA256_RE.fullmatch(evidence_digest)) and bool(rubric_id),
                 "gold record requires evidence digest and rubric id")
        _require(isinstance(record.get("rubric_version"), int)
                 and record["rubric_version"] > 0,
                 "gold record requires positive rubric_version")
        reviewers = record.get("reviewer_labels")
        _require(isinstance(reviewers, list) and len(reviewers) == 2,
                 "gold record requires exactly two reviewer labels")
        reviewer_ids = [str(item.get("reviewer", "")) for item in reviewers]
        _require(len(set(reviewer_ids)) == 2
                 and all(REVIEW_ID_RE.fullmatch(value) for value in reviewer_ids),
                 "gold reviewers must be two independent stable identifiers")
        values = [item.get("value") for item in reviewers]
        _require(all(_is_label_value(value) for value in values),
                 "invalid human reviewer label")
        final = record.get("adjudicated")
        _require(isinstance(final, dict)
                 and REVIEW_ID_RE.fullmatch(str(final.get("rubric_owner", "")))
                 and _is_label_value(final.get("value")),
                 "gold record requires Rubric Owner adjudication")
        key = (evidence_digest, rubric_id)
        _require(key in machine_index, f"missing machine label for {key}")
        expected_keys.add(key)
        reviewer_left.append(values[0])
        reviewer_right.append(values[1])
        adjudicated.append(final["value"])
        machine_values.append(machine_index[key])
    _require(bool(adjudicated), "calibration requires at least one gold record")
    _require(set(machine_index) == expected_keys,
             "machine calibration labels contain unregistered items")
    kappa, reason = _cohen_kappa(adjudicated, machine_values)
    return {
        "schema_version": 1,
        "evaluator": machine.get("evaluator"),
        "evaluator_fingerprint": fingerprint,
        "sample_count": len(adjudicated),
        "human_human_exact": exact_agreement(reviewer_left, reviewer_right),
        "human_machine_exact": exact_agreement(adjudicated, machine_values),
        "unknown_rate": sum(value == "unknown" for value in machine_values)
                        / len(machine_values),
        "human_machine_kappa": kappa,
        "kappa_unavailable_reason": reason,
        "policy": policy,
    }


def cmd_calibrate(args: argparse.Namespace) -> int:
    gold = json.loads(args.gold_set.read_text(encoding="utf-8"))
    machine = json.loads(args.machine_labels.read_text(encoding="utf-8"))
    sys.stdout.buffer.write(normalized_json(compute_calibration(gold, machine)))
    return 0
```

Register the command in `build_parser()`:

```python
    calibrate = commands.add_parser("calibrate", help="calibrate machine labels")
    calibrate.add_argument("gold_set", type=Path)
    calibrate.add_argument("machine_labels", type=Path)
    calibrate.set_defaults(func=cmd_calibrate)
```

The implementation serializes kappa only when at least two classes are
observed and expected agreement is below one. Otherwise it emits `null` plus
`kappa_unavailable_reason`.

- [ ] **Step 4: Implement bounded evaluator projections**

Add tests proving the projection excludes prompts and unrelated evidence and
rejects a captured path that the case did not allowlist:

```python
    def test_evaluator_projection_is_allowlist_only(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        rubric = opi_eval.SubjectiveRubric(
            id="quality.confirmation",
            version=1,
            severity="advisory",
            question="Does the final answer confirm completion?",
            evidence=("final_text",),
            calibration={
                "minimum_samples": 20,
                "minimum_human_human_exact": 0.9,
                "minimum_human_machine_exact": 0.9,
                "maximum_unknown_rate": 0.1,
                "evaluator_fingerprint": "a" * 64,
            },
        )
        case = dataclasses.replace(case, subjective_rubrics=(rubric,))
        evidence = {
            "assistant": {"final_text": "Completed."},
            "trajectory": {"tool_calls": ["read", "write"]},
            "captured": {"files": {"result.txt": {"text": "10\n"},
                                    "ignored.txt": {"text": "do-not-project"}}},
        }
        projection = opi_eval.build_evaluator_input(case, evidence)
        serialized = opi_eval.normalized_json(projection).decode("utf-8")
        self.assertNotIn(case.prompt, serialized)
        self.assertNotIn("do-not-project", serialized)
        self.assertEqual(projection["evidence"]["final_text"], "Completed.")

    def test_evaluator_projection_rejects_undeclared_capture(self) -> None:
        case = opi_eval.load_case(
            Path(__file__).parents[1] / "cases" / "tool_chain.toml"
        )
        rubric = opi_eval.SubjectiveRubric(
            id="quality.secret",
            version=1,
            severity="advisory",
            question="Is the undeclared file acceptable?",
            evidence=("captured:secret.txt",),
            calibration={
                "minimum_samples": 20,
                "minimum_human_human_exact": 0.9,
                "minimum_human_machine_exact": 0.9,
                "maximum_unknown_rate": 0.1,
                "evaluator_fingerprint": "a" * 64,
            },
        )
        case = dataclasses.replace(case, subjective_rubrics=(rubric,))
        with self.assertRaisesRegex(opi_eval.EvalError, "not allowlisted"):
            opi_eval.build_evaluator_input(
                case, {"assistant": {}, "trajectory": {}, "captured": {"files": {}}}
            )
```

Implement the pure projection now. Task 7 binds it to the sealed-bundle loader
and registers the `evaluator-input` CLI command.

```python
def build_evaluator_input(
    case: CaseSpec, evidence: dict[str, Any]
) -> dict[str, Any]:
    requested: set[str] = set()
    rubrics = []
    for rubric in case.subjective_rubrics:
        for source in rubric.evidence:
            if source.startswith("captured:"):
                path = _safe_relative_path(source.removeprefix("captured:"))
                if path not in case.capture_paths:
                    raise EvalError(f"subjective evidence path is not allowlisted: {path}")
            elif source not in {"final_text", "tool_calls"}:
                raise EvalError(f"unsupported subjective evidence source: {source}")
            requested.add(source)
        rubrics.append({
            "id": rubric.id,
            "version": rubric.version,
            "severity": rubric.severity,
            "question": rubric.question,
            "evidence": list(rubric.evidence),
            "calibration": rubric.calibration,
        })
    projected: dict[str, Any] = {}
    for source in sorted(requested):
        if source == "final_text":
            projected[source] = evidence.get("assistant", {}).get("final_text")
        elif source == "tool_calls":
            projected[source] = evidence.get("trajectory", {}).get("tool_calls")
        else:
            path = source.removeprefix("captured:")
            entry = evidence.get("captured", {}).get("files", {}).get(path)
            projected[source] = None if entry is None else entry.get("text")
    base = {
        "schema_version": 1,
        "case": {"id": case.id, "version": case.version,
                 "digest": case.digest},
        "rubrics": sorted(rubrics, key=lambda item: item["id"]),
        "evidence": projected,
    }
    return {**base, "evidence_digest": _sha256_bytes(_canonical_bytes(base))}
```

- [ ] **Step 5: Write the calibration README with exact JSON contracts**

Document:

- two-reviewer independent labels plus Rubric Owner adjudication;
- the gold, machine-label, policy, and calibration-result JSON keys;
- evaluator fingerprint inputs: provider, model, system prompt digest, rubric
  digest, evidence-projection schema, and output-parser schema;
- the skill host computes and injects that fingerprint; evaluator output does
  not choose or attest its own identity;
- exact agreement and optional kappa;
- invalidation rules; and
- the rule that calibrated subjective labels cannot override deterministic
  outcomes.

Use these exact top-level contracts in the README; expand each field with its
validation rule from `compute_calibration`:

```json
{
  "schema_version": 1,
  "policy": {
    "evaluator_fingerprint": "64-lowercase-hex",
    "minimum_samples": 20,
    "minimum_human_human_exact": 0.9,
    "minimum_human_machine_exact": 0.9,
    "maximum_unknown_rate": 0.1
  },
  "records": [{
    "case_id": "case-id",
    "case_version": 1,
    "rubric_id": "rubric-id",
    "rubric_version": 1,
    "evidence_digest": "64-lowercase-hex",
    "reviewer_labels": [
      {"reviewer": "reviewer-a", "value": 1},
      {"reviewer": "reviewer-b", "value": 1}
    ],
    "adjudicated": {"rubric_owner": "rubric-owner", "value": 1}
  }]
}
```

```json
{
  "schema_version": 1,
  "evaluator": {"provider": "provider-id", "model": "model-id"},
  "evaluator_fingerprint": "64-lowercase-hex",
  "records": [{
    "trial_id": "trial-001",
    "evidence_digest": "64-lowercase-hex",
    "labels": {
      "rubric-id": {"value": 1, "evidence": "bounded factual explanation"}
    }
  }]
}
```

The result schema is the exact object returned by `compute_calibration`,
including nullable `human_machine_kappa` and
`kappa_unavailable_reason`.

- [ ] **Step 6: Run tests and the calibration CLI fixture**

Run the unittest command. Expected: all tests pass. Exercise `calibrate` using
temporary JSON files created inside the unittest; do not create workspace-root
scratch files.

Add this fixture-backed CLI test before running the suite:

```python
    def test_calibrate_cli_uses_temporary_gold_and_machine_files(self) -> None:
        fingerprint = "a" * 64
        gold = {
            "schema_version": 1,
            "policy": {
                "evaluator_fingerprint": fingerprint,
                "minimum_samples": 2,
                "minimum_human_human_exact": 1.0,
                "minimum_human_machine_exact": 1.0,
                "maximum_unknown_rate": 0.0,
            },
            "records": [
                {
                    "case_id": "case", "case_version": 1,
                    "rubric_id": "quality.atomic", "rubric_version": 1,
                    "evidence_digest": "1" * 64,
                    "reviewer_labels": [
                        {"reviewer": "reviewer-a", "value": 1},
                        {"reviewer": "reviewer-b", "value": 1},
                    ],
                    "adjudicated": {"rubric_owner": "rubric-owner", "value": 1},
                },
                {
                    "case_id": "case", "case_version": 1,
                    "rubric_id": "quality.atomic", "rubric_version": 1,
                    "evidence_digest": "2" * 64,
                    "reviewer_labels": [
                        {"reviewer": "reviewer-a", "value": 0},
                        {"reviewer": "reviewer-b", "value": 0},
                    ],
                    "adjudicated": {"rubric_owner": "rubric-owner", "value": 0},
                },
            ],
        }
        machine = {
            "schema_version": 1,
            "evaluator": {"provider": "mock", "model": "mock"},
            "evaluator_fingerprint": fingerprint,
            "records": [
                {"trial_id": "trial-001", "evidence_digest": "1" * 64,
                 "labels": {"quality.atomic": {"value": 1, "evidence": "yes"}}},
                {"trial_id": "trial-002", "evidence_digest": "2" * 64,
                 "labels": {"quality.atomic": {"value": 0, "evidence": "no"}}},
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gold_path = root / "gold.json"
            machine_path = root / "machine.json"
            gold_path.write_text(json.dumps(gold), encoding="utf-8")
            machine_path.write_text(json.dumps(machine), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(MODULE_PATH), "calibrate",
                 str(gold_path), str(machine_path)],
                check=True, capture_output=True, text=True, encoding="utf-8",
            )
        result = json.loads(completed.stdout)
        self.assertEqual(result["sample_count"], 2)
        self.assertEqual(result["human_machine_exact"], 1.0)
```

- [ ] **Step 7: Conditional checkpoint commit**

Only with explicit authorization, stage the two scripts and calibration README
by exact path and commit:

```text
git commit -m "feat(opi-eval): calibrate subjective diagnostics"
```

### Task 6: Add authority-separated reports and append-only history

**Files:**
- Modify: `.claude/skills/opi-eval/scripts/opi_eval.py`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`
- Modify: `.claude/skills/opi-eval/references/report-template.md:1-143`
- Modify: `docs/eval/README.md:1-82`

- [ ] **Step 1: Write failing report tests**

Add a test that renders a failed deterministic outcome and an uncalibrated
machine pass. Assert the report remains `FAIL`, contains separate
`Authoritative Outcomes` and `Uncalibrated Diagnostics` headings, and does not
contain a configured secret canary.

Add a history test that appends two records, parses both JSONL lines, and proves
the first line is unchanged.

```python
class ReportTests(unittest.TestCase):
    def test_uncalibrated_pass_cannot_hide_deterministic_fail(self) -> None:
        report = opi_eval.render_report(
            run={"run_id": "run-1", "commit": "fixture", "dirty": False},
            grades={"tool_chain": {"trial-001": {"verdict": "FAIL", "results": {}}}},
            machine_labels={
                "evaluator_fingerprint": "fingerprint-a",
                "labels": {"style.clear": {"value": 1, "evidence": "canary-free"}},
            },
            calibration=None,
            attribution=(),
            sensitive_values=(),
        )
        self.assertIn("**Verdict:** FAIL", report)
        self.assertIn("## Authoritative Outcomes", report)
        self.assertIn("## Uncalibrated Diagnostics", report)

    def test_report_blocks_configured_sensitive_value(self) -> None:
        with self.assertRaisesRegex(opi_eval.EvalError, "sensitive value"):
            opi_eval.render_report(
                run={"run_id": "run-1", "commit": "fixture", "dirty": False},
                grades={"candy": {"trial-001": {"verdict": "PASS", "results": {}}}},
                machine_labels={
                    "evaluator_fingerprint": "fingerprint-a",
                    "labels": {"style.clear": {"value": 1, "evidence": "canary-secret"}},
                },
                calibration=None,
                attribution=(),
                sensitive_values=("canary-secret",),
            )

    def test_history_append_preserves_prior_line(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "history.jsonl"
            first = {"schema_version": 1, "run_id": "run-1", "verdict": "PASS"}
            second = {"schema_version": 1, "run_id": "run-2", "verdict": "FAIL"}
            opi_eval.append_history(path, first)
            original = path.read_bytes()
            opi_eval.append_history(path, second)
            lines = path.read_bytes().splitlines(keepends=True)
            self.assertEqual(lines[0], original)
            self.assertEqual(json.loads(lines[1]), second)

    def test_first_run_is_not_an_implicit_efficiency_baseline(self) -> None:
        candidate = {"control_coverage": "complete", "control_fingerprint": "same"}
        self.assertEqual(opi_eval.efficiency_baseline_status(candidate, None), "unknown")

    def test_only_promoted_matching_baseline_is_comparable(self) -> None:
        candidate = {"control_coverage": "complete", "control_fingerprint": "same"}
        baseline = {
            "baseline_status": "promoted",
            "control_coverage": "complete",
            "control_fingerprint": "same",
        }
        self.assertEqual(
            opi_eval.efficiency_baseline_status(candidate, baseline), "comparable"
        )
```

- [ ] **Step 2: Run tests and verify report failures**

Expected: missing `render_report` and `append_history`.

- [ ] **Step 3: Implement deterministic report rendering**

`render_report` takes a run manifest, trial grades, optional validated machine
labels, optional calibration, and attribution records. It renders sections in
this fixed order:

1. Summary and authoritative verdict;
2. Evidence coverage;
3. Authoritative Outcomes;
4. Deterministic Trajectory Findings;
5. Calibrated Subjective Findings;
6. Uncalibrated Diagnostics;
7. Efficiency and baseline coverage;
8. Attribution, split into Observed and Inferred;
9. Environment and artifact digests.

Sort cases, trials, and rubric IDs before rendering. Do not include raw prompts,
full tool results, absolute paths, environment variables, or captured file text.

Implement the fixed section order with a list of lines, not a template engine:

```python
VERDICT_ORDER = {"PASS": 0, "DEGRADED": 1, "INCONCLUSIVE": 2,
                 "FAIL": 3, "ERROR": 4}


def _overall_verdict(grades: dict[str, dict[str, dict[str, Any]]]) -> str:
    values = [trial["verdict"] for case in grades.values() for trial in case.values()]
    if not values:
        return "ERROR"
    return max(values, key=lambda value: VERDICT_ORDER[value])


def efficiency_baseline_status(
    candidate: dict[str, Any], baseline: dict[str, Any] | None
) -> str:
    if baseline is None or baseline.get("baseline_status") != "promoted":
        return "unknown"
    if candidate.get("control_coverage") != "complete" or baseline.get(
        "control_coverage"
    ) != "complete":
        return "unknown"
    if candidate.get("control_fingerprint") != baseline.get("control_fingerprint"):
        return "incompatible"
    return "comparable"


def render_report(
    run: dict[str, Any],
    grades: dict[str, dict[str, dict[str, Any]]],
    machine_labels: dict[str, Any] | None,
    calibration: dict[str, Any] | None,
    attribution: tuple[dict[str, Any], ...],
    sensitive_values: tuple[str, ...],
) -> str:
    verdict = _overall_verdict(grades)
    fingerprint = "" if machine_labels is None else str(
        machine_labels.get("evaluator_fingerprint", "")
    )
    authority = calibration_authority(calibration, fingerprint)
    lines = [
        f"# opi local eval — {run['run_id']}", "",
        f"**Verdict:** {verdict}", "",
        "## Summary", "",
    ]
    for case_id, case in sorted(run.get("cases", {}).items()):
        lines.append(f"- `{case_id}` user value: {case['user_value']}")
    if not run.get("cases"):
        lines.append("No case metadata was supplied.")
    lines.extend([
        "",
        "## Evidence Coverage", "",
        "Evidence is referenced by sealed bundle digests.", "",
        "## Authoritative Outcomes", "",
    ])
    for case_id in sorted(grades):
        for trial_id in sorted(grades[case_id]):
            lines.append(f"- `{case_id}/{trial_id}`: {grades[case_id][trial_id]['verdict']}")
    lines.extend(["", "## Deterministic Trajectory Findings", "",
                  "See per-assertion results in the grade artifacts.", "",
                  "## Calibrated Subjective Findings", ""])
    if authority == "calibrated" and machine_labels is not None:
        for rubric_id in sorted(machine_labels.get("labels", {})):
            label = machine_labels["labels"][rubric_id]
            lines.append(f"- `{rubric_id}`: {label['value']} — {label['evidence']}")
    else:
        lines.append("None.")
    lines.extend(["", "## Uncalibrated Diagnostics", ""])
    if machine_labels is not None and authority == "diagnostic":
        for rubric_id in sorted(machine_labels.get("labels", {})):
            label = machine_labels["labels"][rubric_id]
            lines.append(f"- `{rubric_id}`: {label['value']} — {label['evidence']}")
    else:
        lines.append("None.")
    lines.extend(["", "## Efficiency and Baseline Coverage", ""])
    for measurement in run.get("measurements", []):
        lines.append(
            f"- `{measurement['case_id']}/{measurement['trial_id']}`: "
            f"duration={measurement.get('duration_seconds', 'unknown')}, "
            f"tokens={measurement.get('tokens', 'unknown')}"
        )
    if not run.get("measurements"):
        lines.append("No resource measurements were supplied.")
    lines.extend(["No efficiency claim is made without an explicit matching baseline.", "",
                  "## Attribution", "", "### Observed", ""])
    observed = [item for item in attribution if item.get("kind") == "observed"]
    inferred = [item for item in attribution if item.get("kind") == "inferred"]
    lines.extend(f"- {item['category']}: {item['detail']}" for item in observed)
    if not observed:
        lines.append("None.")
    lines.extend(["", "### Inferred", ""])
    lines.extend(f"- {item['category']}: {item['detail']}" for item in inferred)
    if not inferred:
        lines.append("None.")
    lines.extend(["", "## Environment and Artifact Digests", "",
                  f"- Commit: `{run['commit']}`",
                  f"- Dirty worktree: `{str(run['dirty']).lower()}`", ""])
    report = "\n".join(lines)
    _scan_text(report, Path.cwd(), sensitive_values)
    return report
```

- [ ] **Step 4: Implement schema-versioned append-only history**

`append_history(path, record)` validates `schema_version = 1`, writes one
sorted compact JSON object plus newline using append mode, flushes, and calls
`os.fsync`. The record contains only run ID, commit, dirty flag, date, requested
and actual model identities, case verdicts, trial counts, calibration authority,
and bundle/report digests.

```python
def append_history(path: Path, record: dict[str, Any]) -> None:
    if record.get("schema_version") != 1:
        raise EvalError("history schema_version must be 1")
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = normalized_json(record)
    with path.open("ab") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
```

- [ ] **Step 5: Update the report template and docs/eval contract**

Rewrite `report-template.md` to the nine fixed sections above and show all five
verdicts. Rewrite `docs/eval/README.md` so it:

- declares report/history schema version 1;
- distinguishes reports from local raw bundles;
- documents `target/opi-eval/RUN_ID` retention and digest references;
- defines `INCONCLUSIVE` separately from `ERROR`;
- forbids best-of trial reporting;
- states that missing or deleted bundles make prior summaries
  `evidence-unavailable`, not reproducible; and
- removes the obsolete cursor-subagent example.

- [ ] **Step 6: Run report tests and documentation checks**

Run:

```text
python -m unittest .claude/skills/opi-eval/scripts/test_opi_eval.py
python scripts/opi-doc-check.py
git diff --check
```

Expected: tests pass, documentation contracts report `PASS`, and diff check
reports no whitespace errors.

- [ ] **Step 7: Conditional checkpoint commit**

Only with explicit authorization, stage the two scripts, report template, and
`docs/eval/README.md` by exact path and commit:

```text
git commit -m "feat(opi-eval): report authoritative local evidence"
```

### Task 7: Add the saved offline bundle and end-to-end regrade test

**Files:**
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/cases/tool_chain.toml`
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/manifest.json`
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/trials/trial-001/output.ndjson`
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/trials/trial-001/trial.json`
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/trials/trial-001/capture-manifest.json`
- Create: `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/trials/trial-001/captured/result.txt`
- Modify: `.claude/skills/opi-eval/scripts/test_opi_eval.py`

- [ ] **Step 1: Create the redacted fixture bundle**

Use run ID `fixture-tool-chain`, trial ID `trial-001`, model `mock:model`, clean
commit marker `fixture`, and no absolute paths. The NDJSON contains ordered read
and write start/end events, one final `MessageEnd`, and one `session_summary`.
The captured `result.txt` contains exactly `10` plus one LF; its SHA-256 is:

```text
917df3320d778ddbaa5c5c7742bc4046bf803c36ed2b050f30844ed206783469
```

The capture manifest records path `result.txt`, size `3`, UTF-8 media type, and
that digest. Compute and insert the real case digest using `load_case` rather
than inventing a value. Copy `cases/tool_chain.toml` byte-for-byte from the
authoritative manifest and record both its parsed case digest and raw artifact
SHA-256 in `manifest.json`.

- [ ] **Step 2: Write the failing fixture regrade test**

```python
class OfflineBundleTests(unittest.TestCase):
    def test_saved_tool_chain_bundle_regrades_to_pass(self) -> None:
        bundle = Path(__file__).with_name("fixtures") / "tool_chain_bundle"
        first = opi_eval.grade_bundle(bundle)
        second = opi_eval.grade_bundle(bundle)
        self.assertEqual(first["cases"]["tool_chain"]["trials"]["trial-001"]["verdict"],
                         "PASS")
        self.assertEqual(opi_eval.normalized_json(first),
                         opi_eval.normalized_json(second))
```

- [ ] **Step 3: Run the test and verify `grade_bundle` is missing**

Expected: one new error.

- [ ] **Step 4: Implement `grade_bundle` and the CLI `grade` command**

Add the verified artifact loader and grader below. A digest mismatch raises
`EvalError`; it never becomes a rubric `unknown`. `PASS` and `DEGRADED` trials
count toward `minimum_pass_rate` because both satisfy every required assertion;
an advisory failure still keeps the case verdict `DEGRADED`.

```python
def _bundle_path(root: Path, relative: str) -> Path:
    normalized = _safe_relative_path(relative)
    resolved_root = root.resolve()
    lexical = resolved_root / Path(normalized)
    cursor = resolved_root
    for part in Path(normalized).parts:
        cursor /= part
        if cursor.is_symlink():
            raise EvalError(f"bundle path traverses a symlink: {relative}")
    path = lexical.resolve()
    if resolved_root not in path.parents:
        raise EvalError(f"bundle path escaped root: {relative}")
    return path


def _verified_bytes(root: Path, relative: str, expected_sha256: str) -> bytes:
    _require(bool(SHA256_RE.fullmatch(expected_sha256)),
             f"invalid artifact digest: {relative}")
    path = _bundle_path(root, relative)
    _require(path.is_file() and not path.is_symlink(),
             f"bundle artifact is missing or not regular: {relative}")
    data = path.read_bytes()
    _require(_sha256_bytes(data) == expected_sha256,
             f"bundle artifact digest mismatch: {relative}")
    return data


def _load_bundle_cases(
    bundle_root: Path, manifest: dict[str, Any]
) -> dict[str, CaseSpec]:
    cases: dict[str, CaseSpec] = {}
    for case_id, entry in sorted(manifest.get("cases", {}).items()):
        artifact = str(entry.get("artifact", ""))
        data = _verified_bytes(
            bundle_root, artifact, str(entry.get("artifact_sha256", ""))
        )
        case_path = _bundle_path(bundle_root, artifact)
        case = load_case(case_path)
        _require(case.id == case_id
                 and case.version == entry.get("version")
                 and case.digest == entry.get("digest"),
                 f"case snapshot mismatch: {case_id}")
        _require(data == case_path.read_bytes(), f"case snapshot changed: {case_id}")
        cases[case_id] = case
    _require(bool(cases), "bundle contains no case snapshots")
    return cases


def load_trial_evidence(
    bundle_root: Path,
    trial_record: dict[str, Any],
    case: CaseSpec,
) -> dict[str, Any]:
    trial_id = str(trial_record.get("trial_id", ""))
    _require(bool(TRIAL_ID_RE.fullmatch(trial_id)), "invalid bundled trial id")
    trial_root = _bundle_path(bundle_root, f"trials/{trial_id}")
    artifact_digests = trial_record.get("artifact_digests")
    _require(isinstance(artifact_digests, dict) and artifact_digests,
             f"trial has no sealed artifacts: {trial_id}")
    verified: dict[str, bytes] = {}
    for name, digest in sorted(artifact_digests.items()):
        _safe_relative_path(str(name))
        verified[str(name)] = _verified_bytes(
            trial_root, str(name), str(digest)
        )
    _require("trial.json" in verified, f"trial metadata is missing: {trial_id}")
    trial = json.loads(verified["trial.json"])
    _require(trial.get("schema_version") == 1,
             f"unsupported trial schema: {trial_id}")
    _require(trial.get("case_id") == case.id
             and trial.get("case_version") == case.version
             and trial.get("case_digest") == case.digest,
             f"trial case binding mismatch: {trial_id}")
    if trial.get("status") == "ERROR":
        return {"error": trial.get("error", {"kind": "execution"})}
    _require("output.ndjson" in verified
             and "capture-manifest.json" in verified,
             f"trial evidence artifacts are missing: {trial_id}")
    try:
        output_text = verified["output.ndjson"].decode("utf-8")
    except UnicodeDecodeError as error:
        raise EvalError(f"trial NDJSON is not UTF-8: {trial_id}") from error
    capture_manifest = json.loads(verified["capture-manifest.json"])
    _require(capture_manifest.get("schema_version") == 1,
             f"unsupported capture schema: {trial_id}")
    files: dict[str, dict[str, Any]] = {}
    for relative, entry in sorted(capture_manifest.get("files", {}).items()):
        _require(relative in case.capture_paths,
                 f"captured path was not allowlisted: {relative}")
        captured_data = _verified_bytes(
            trial_root,
            f"captured/{relative}",
            str(entry.get("sha256", "")),
        )
        _require(len(captured_data) == entry.get("size"),
                 f"captured size mismatch: {relative}")
        try:
            text = captured_data.decode("utf-8")
        except UnicodeDecodeError as error:
            raise EvalError(f"captured file is not UTF-8: {relative}") from error
        files[relative] = {
            "size": len(captured_data),
            "sha256": str(entry["sha256"]),
            "text": text,
        }
    return {
        **project_ndjson(output_text),
        "process": {"exit_code": trial.get("exit_code")},
        "captured": {"files": files},
    }


def grade_bundle(bundle_root: Path) -> dict[str, Any]:
    manifest_path = bundle_root / "manifest.json"
    _require(manifest_path.is_file() and not manifest_path.is_symlink(),
             "bundle manifest is missing")
    manifest_bytes = manifest_path.read_bytes()
    manifest = json.loads(manifest_bytes)
    _require(manifest.get("schema_version") == RUN_SCHEMA_VERSION,
             "unsupported run schema")
    _require(manifest.get("status") == "sealed", "run bundle is not sealed")
    cases = _load_bundle_cases(bundle_root, manifest)
    result_cases: dict[str, Any] = {
        case_id: {"trials": {}} for case_id in sorted(cases)
    }
    for trial_record in sorted(
        manifest.get("trials", []), key=lambda item: int(item["ordinal"])
    ):
        case_id = str(trial_record.get("case_id", ""))
        _require(case_id in cases, f"trial references unknown case: {case_id}")
        trial_id = str(trial_record.get("trial_id", ""))
        if not trial_record.get("artifact_digests"):
            grade = {
                "schema_version": GRADE_SCHEMA_VERSION,
                "case_id": case_id,
                "case_version": cases[case_id].version,
                "case_digest": cases[case_id].digest,
                "verdict": "ERROR",
                "results": {},
                "error": {"kind": "evidence-unavailable",
                          "message": str(trial_record.get("error", ""))},
            }
        else:
            evidence = load_trial_evidence(bundle_root, trial_record, cases[case_id])
            if "error" in evidence:
                grade = {
                    "schema_version": GRADE_SCHEMA_VERSION,
                    "case_id": case_id,
                    "case_version": cases[case_id].version,
                    "case_digest": cases[case_id].digest,
                    "verdict": "ERROR",
                    "results": {},
                    "error": evidence["error"],
                }
            else:
                grade = grade_evidence(cases[case_id], evidence)
        result_cases[case_id]["trials"][trial_id] = grade
    for case_id, case_result in result_cases.items():
        case = cases[case_id]
        verdicts = [item["verdict"] for item in case_result["trials"].values()]
        _require(bool(verdicts), f"case has no trials: {case_id}")
        pass_count = sum(value in {"PASS", "DEGRADED"} for value in verdicts)
        pass_rate = pass_count / len(verdicts)
        minimum = float(case.execution["minimum_pass_rate"])
        if pass_rate < minimum:
            if "ERROR" in verdicts:
                verdict = "ERROR"
            elif "FAIL" in verdicts:
                verdict = "FAIL"
            else:
                verdict = "INCONCLUSIVE"
        elif any(value != "PASS" for value in verdicts):
            verdict = "DEGRADED"
        else:
            verdict = "PASS"
        case_result.update({
            "verdict": verdict,
            "pass_count": pass_count,
            "trial_count": len(verdicts),
            "pass_rate": pass_rate,
            "minimum_pass_rate": minimum,
        })
    overall = max(
        (entry["verdict"] for entry in result_cases.values()),
        key=lambda value: VERDICT_ORDER[value],
    )
    return {
        "schema_version": GRADE_SCHEMA_VERSION,
        "run_id": manifest.get("run_id"),
        "run_manifest_sha256": _sha256_bytes(manifest_bytes),
        "verdict": overall,
        "cases": result_cases,
    }


def cmd_grade(args: argparse.Namespace) -> int:
    grade = grade_bundle(args.run_bundle)
    payload = normalized_json(grade)
    if args.output is not None:
        atomic_json(args.output, grade)
    sys.stdout.buffer.write(payload)
    return int(grade["verdict"] in {"FAIL", "INCONCLUSIVE", "ERROR"})
```

Register the command:

```python
    grade = commands.add_parser("grade", help="regrade a sealed run bundle")
    grade.add_argument("run_bundle", type=Path)
    grade.add_argument("--output", type=Path)
    grade.set_defaults(func=cmd_grade)
```

Bind the Task 5 projection to sealed evidence now that the verified loader
exists:

```python
def cmd_evaluator_input(args: argparse.Namespace) -> int:
    manifest = json.loads((args.run_bundle / "manifest.json").read_text(
        encoding="utf-8"
    ))
    _require(manifest.get("schema_version") == RUN_SCHEMA_VERSION
             and manifest.get("status") == "sealed",
             "evaluator input requires a sealed run bundle")
    cases = _load_bundle_cases(args.run_bundle, manifest)
    projections = []
    for trial_record in sorted(
        manifest.get("trials", []), key=lambda item: int(item["ordinal"])
    ):
        case = cases[str(trial_record["case_id"])]
        if not case.subjective_rubrics:
            continue
        evidence = load_trial_evidence(args.run_bundle, trial_record, case)
        if "error" in evidence:
            continue
        projections.append({
            "trial_id": trial_record["trial_id"],
            "input": build_evaluator_input(case, evidence),
        })
    sys.stdout.buffer.write(normalized_json({
        "schema_version": 1,
        "run_id": manifest.get("run_id"),
        "trials": projections,
    }))
    return 0
```

Register it in `build_parser()`:

```python
    evaluator_input = commands.add_parser(
        "evaluator-input", help="project bounded subjective evidence"
    )
    evaluator_input.add_argument("run_bundle", type=Path)
    evaluator_input.set_defaults(func=cmd_evaluator_input)
```

Finally bind Task 6 reporting to the same verified bundle loader. Machine-label
input uses the calibration batch shape: common evaluator metadata plus
`records`, each with `trial_id`, `evidence_digest`, and `labels`.

```python
def _atomic_text(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f"{path.name}.tmp-{os.getpid()}-{uuid.uuid4().hex}")
    temporary.write_text(value, encoding="utf-8", newline="")
    os.replace(temporary, path)


def cmd_report(args: argparse.Namespace) -> int:
    manifest = json.loads((args.run_bundle / "manifest.json").read_text(
        encoding="utf-8"
    ))
    cases = _load_bundle_cases(args.run_bundle, manifest)
    grade = grade_bundle(args.run_bundle)
    sensitive_values, _sensitive_digest = _load_sensitive_values(
        args.sensitive_values_file
    )
    trial_by_id = {
        str(item["trial_id"]): item for item in manifest.get("trials", [])
    }
    flattened_labels: dict[str, Any] = {}
    machine_for_report: dict[str, Any] | None = None
    if args.machine_labels is not None:
        machine_batch = json.loads(args.machine_labels.read_text(encoding="utf-8"))
        _require(machine_batch.get("schema_version") == 1,
                 "machine-label batch schema_version must be 1")
        for record in machine_batch.get("records", []):
            trial_id = str(record.get("trial_id", ""))
            _require(trial_id in trial_by_id,
                     f"machine labels reference unknown trial: {trial_id}")
            trial = trial_by_id[trial_id]
            case = cases[str(trial["case_id"])]
            evidence = load_trial_evidence(args.run_bundle, trial, case)
            _require("error" not in evidence,
                     f"machine labels reference an error trial: {trial_id}")
            projection = build_evaluator_input(case, evidence)
            validated = validate_machine_labels(
                {
                    "schema_version": 1,
                    "evaluator": machine_batch.get("evaluator"),
                    "evaluator_fingerprint": machine_batch.get(
                        "evaluator_fingerprint"
                    ),
                    "evidence_digest": record.get("evidence_digest"),
                    "labels": record.get("labels"),
                },
                case.subjective_rubrics,
                projection["evidence_digest"],
            )
            for rubric_id, label in validated["labels"].items():
                flattened_labels[f"{trial_id}/{rubric_id}"] = label
        machine_for_report = {
            "evaluator_fingerprint": machine_batch.get("evaluator_fingerprint"),
            "labels": flattened_labels,
        }
    calibration = (
        None if args.calibration is None
        else json.loads(args.calibration.read_text(encoding="utf-8"))
    )
    attribution_values = (
        [] if args.attribution is None
        else json.loads(args.attribution.read_text(encoding="utf-8"))
    )
    _require(isinstance(attribution_values, list), "attribution must be a JSON array")
    grades_for_report = {
        case_id: entry["trials"] for case_id, entry in grade["cases"].items()
    }
    run_for_report = dict(manifest)
    run_for_report["measurements"] = []
    for trial in manifest.get("trials", []):
        artifacts = trial.get("artifact_digests", {})
        if "trial.json" not in artifacts:
            continue
        trial_id = str(trial["trial_id"])
        trial_data = json.loads(_verified_bytes(
            _bundle_path(args.run_bundle, f"trials/{trial_id}"),
            "trial.json",
            str(artifacts["trial.json"]),
        ))
        evidence = load_trial_evidence(
            args.run_bundle, trial, cases[str(trial["case_id"])]
        )
        summary = evidence.get("session_summary", {})
        run_for_report["measurements"].append({
            "case_id": trial["case_id"],
            "trial_id": trial_id,
            "duration_seconds": trial_data.get("duration_seconds"),
            "tokens": summary.get("tokens") if isinstance(summary, dict) else None,
        })
    report = render_report(
        run=run_for_report,
        grades=grades_for_report,
        machine_labels=machine_for_report,
        calibration=calibration,
        attribution=tuple(attribution_values),
        sensitive_values=sensitive_values,
    )
    if args.output is not None:
        _atomic_text(args.output, report)
    if args.history is not None:
        append_history(args.history, {
            "schema_version": 1,
            "run_id": manifest["run_id"],
            "commit": manifest["commit"],
            "dirty": manifest["dirty"],
            "date": manifest["completed_at"],
            "requested_model": manifest.get("requested_model"),
            "actual_models": manifest.get("actual_models", []),
            "case_verdicts": {
                case_id: value["verdict"]
                for case_id, value in grade["cases"].items()
            },
            "trial_counts": {
                case_id: value["trial_count"]
                for case_id, value in grade["cases"].items()
            },
            "calibration_authority": calibration_authority(
                calibration,
                "" if machine_for_report is None else str(
                    machine_for_report["evaluator_fingerprint"]
                ),
            ),
            "bundle_digest": grade["run_manifest_sha256"],
            "report_digest": _sha256_bytes(report.encode("utf-8")),
        })
    sys.stdout.write(report)
    return int(grade["verdict"] in {"FAIL", "INCONCLUSIVE", "ERROR"})
```

Register it in `build_parser()`:

```python
    report = commands.add_parser("report", help="render a sealed run report")
    report.add_argument("run_bundle", type=Path)
    report.add_argument("--machine-labels", type=Path)
    report.add_argument("--calibration", type=Path)
    report.add_argument("--attribution", type=Path)
    report.add_argument("--sensitive-values-file", type=Path)
    report.add_argument("--output", type=Path)
    report.add_argument("--history", type=Path)
    report.set_defaults(func=cmd_report)
```

- [ ] **Step 5: Run the exact offline verification command**

Run:

```text
python .claude/skills/opi-eval/scripts/opi_eval.py grade .claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle
```

Expected: exit zero and normalized JSON containing
`"tool_chain"`, `"trial-001"`, and `"verdict":"PASS"`.

- [ ] **Step 6: Conditional checkpoint commit**

Only with explicit authorization, stage the six fixture files and the two
script files by exact path and commit:

```text
git commit -m "test(opi-eval): add offline regrade bundle"
```

### Task 8: Rewrite the skill workflow and authoring references

**Files:**
- Modify: `.claude/skills/opi-eval/SKILL.md:1-261`
- Modify: `.claude/skills/opi-eval/references/test-cases.md:1-161`
- Modify: `.claude/skills/opi-eval/references/evaluator-prompt.md:1-155`
- Modify: `.claude/skills/opi-eval/references/report-template.md`
- Modify: `.claude/skills/opi-eval/agents/openai.yaml`

- [ ] **Step 1: Rewrite `SKILL.md` around the executable evidence flow**

Keep `disable-model-invocation: true`. Replace the prose-defined-case workflow
with these ordered phases and explicit completion criteria:

1. `validate` selected pinned manifests before any build or provider call;
2. `run` a fresh release binary only after explicit user invocation;
3. seal evidence before grading or evaluator dispatch;
4. run deterministic `grade` first;
5. emit `evaluator-input` and dispatch a readonly evaluator only when the
   selected cases contain subjective rubrics;
6. validate machine labels and determine calibration authority;
7. render report/history with deterministic and diagnostic sections separated;
8. preserve normalized regression findings without modifying product source.

State that an uncalibrated evaluator is diagnostic, an evaluator cannot
override deterministic outcomes, and a real-provider failure in one case does
not skip remaining cases. Remove the old six broad dimensions as direct score
authority; retain them only as authoring topics that must be decomposed into
atomic assertions or rubrics.

- [ ] **Step 2: Convert `test-cases.md` into a non-duplicating authoring guide**

The file must contain:

- links to all three TOML manifests;
- the lifecycle `candidate -> pinned -> retired`;
- supported assertion kinds and their required fields;
- source/review/privacy requirements;
- a rule requiring at least one deterministic required outcome for pinned
  cases;
- a rule prohibiting arbitrary setup/grader shell commands; and
- commands for `validate` and fixture-backed offline `grade`.

Remove all copied prompts and expected-answer prose after verifying they exist
in the TOML manifests.

- [ ] **Step 3: Replace the evaluator prompt with the atomic label protocol**

Require JSON-only output with:

```json
{
  "labels": {
    "rubric-id": {"value": "unknown", "evidence": "bounded factual explanation"}
  }
}
```

Allowed label values are JSON numbers `1`, `0`, and the string `"unknown"`.
The evaluator must label only supplied rubric IDs, cite only projected evidence,
avoid code recommendations, and return `unknown` when evidence is insufficient.
The skill host wraps this body with schema version, evaluator provider/model,
the host-computed evaluator fingerprint, and the supplied evidence digest before
calling `validate_machine_labels`; model output never supplies those authority
fields.

- [ ] **Step 4: Synchronize the Codex sidecar without changing invocation policy**

Keep `policy.allow_implicit_invocation: false`. Update only the display text and
default prompt needed to describe selected cases/model and the new
deterministic-first workflow. Preserve any unrelated user edits already present
in `.claude/skills/opi-eval/agents/openai.yaml`; if overlap cannot be resolved
surgically, stop and ask the user rather than overwriting it.

- [ ] **Step 5: Run documentation and skill-contract checks**

Run:

```text
python scripts/opi-doc-check.py
git diff --check
```

Expected: documentation contracts `PASS`; no whitespace errors.

- [ ] **Step 6: Conditional checkpoint commit**

Only with explicit authorization, stage the skill, three references, report
template, and sidecar by exact path and commit:

```text
git commit -m "docs(opi-eval): define deterministic local workflow"
```

### Task 9: Run the complete local-eval acceptance gate

**Files:**
- Verify only; fix only task-owned files listed in Tasks 1-8.

- [ ] **Step 1: Inspect scope before verification**

Run `git status --short`. Confirm every path changed by this implementation is
listed in the File map and identify unrelated pre-existing changes without
staging or modifying them.

- [ ] **Step 2: Run the full Python suite**

Run:

```text
python -m unittest .claude/skills/opi-eval/scripts/test_opi_eval.py
```

Expected: all tests pass; no provider, Cargo build, or network call occurs.

- [ ] **Step 3: Validate the authoritative case registry**

Run:

```text
python .claude/skills/opi-eval/scripts/opi_eval.py validate
```

Expected: `validated 3 cases` and exit zero.

- [ ] **Step 4: Regrade the saved bundle twice**

Run the Task 7 grade command twice and compare stdout bytes. Expected: both
outputs are byte-identical and report `PASS`.

- [ ] **Step 5: Run repository documentation checks**

Run:

```text
python scripts/opi-doc-check.py
git diff --check
```

Expected: documentation contracts `PASS`; no whitespace errors. Do not run a
Rust build because this delivery changes only Python skill tooling, manifests,
fixtures, and documentation.

- [ ] **Step 6: Review acceptance evidence against the design**

Record evidence for all twelve acceptance criteria from
`docs/superpowers/specs/2026-08-11-opi-local-eval-foundation-design.md`. In
particular, cite the wrong-`result.txt` test, missing-evidence test,
uncalibrated-authority test, fingerprint-invalidation test, leakage tests, and
byte-stable offline regrade test.

- [ ] **Step 7: Conditional final implementation commit**

Only if the user authorized commits and task checkpoints were not already
committed, stage each task-owned path explicitly and commit:

```text
git commit -m "feat(opi-eval): establish trustworthy local evaluation"
```

Never stage `.opi-impl-state.json` in this implementation commit. When execution
is admitted through `opi-implement`, follow its separate task-commit and ledger-
checkpoint commit protocol.
