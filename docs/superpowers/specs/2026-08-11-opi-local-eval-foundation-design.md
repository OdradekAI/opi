# Opi Local Eval Foundation Design

**Status:** Approved design

**Date:** 2026-08-11

**Scope:** Make the project-local `opi-eval` workflow produce trustworthy,
replayable local-smoke evidence through deterministic outcome grading, atomic
rubrics, explicit evaluator calibration, and a reviewed case lifecycle.

## Authority and inputs

This design is a finite implementation design under the durable direction in
`docs/opi-spec.md`. It does not revise that parent specification.

The relevant parent requirements are:

- `CTRL-002`: evidence retains sufficient provenance for offline verification;
- `CTRL-004`: native or deterministic graders own headline outcomes, while an
  LLM judge is separately labelled;
- `CTRL-006`: reports are reproducible from immutable run bundles; and
- `CTRL-007`: datasets, sandboxes, schedulers, and external benchmark policy
  stay outside the evaluation module.

The following are non-normative inputs:

- `.claude/skills/opi-eval/SKILL.md` and its three existing local cases;
- `docs/research/2026-07-10-opi-agent-benchmark-plan.zh.md`;
- the approved follow-up boundary in
  `docs/superpowers/specs/2026-08-11-opi-skill-contract-consistency-design.md`;
  and
- Meituan's article
  [Agent评测漫谈 —— 由浅入深讲解Agent评测](https://tech.meituan.com/2026/08/07/Agent-Evaluation.html),
  particularly its outcome/trajectory distinction, atomic-rubric guidance,
  human/machine calibration, and Good Case/Bad Case feedback loop.

## Problem

The current `opi-eval` workflow is a useful real-provider smoke test, but it is
not yet a trustworthy local evaluation foundation:

- its three cases are embedded in a prose reference rather than versioned
  machine-readable manifests;
- the `tool_chain` case requires `result.txt` to contain `10`, but the generic
  extraction flow does not require an independent post-run workspace check;
- broad `PASS`/`DEGRADED`/`FAIL` judgments contain subjective phrases such as
  "minor redundancy" and "obviously anomalous";
- an LLM evaluator produces the aggregate assessment without a versioned human
  gold set or a measured human/machine agreement gate;
- the first observed efficiency run implicitly becomes a baseline;
- there is no reviewed lifecycle for turning explicit, redacted failures into
  pinned regression cases; and
- `docs/eval/` describes reports and history but contains no durable run
  evidence.

Building external benchmark adapters before closing these gaps would produce a
larger runner without first proving that Opi can create, grade, replay, and
govern a small local case set reliably.

## Goals

1. Make final environment state the primary local-smoke outcome authority.
2. Represent every case as a versioned, validated, machine-readable manifest.
3. Reduce judgment ambiguity through atomic `1 | 0 | unknown` assertions and
   rubrics.
4. Keep deterministic grading, subjective grading, calibration, and reporting
   as separate responsibilities.
5. Preserve sufficient immutable evidence to regrade a run offline.
6. Admit cases to the default suite only through explicit human review.
7. Keep sensitive raw evidence local and out of Git.
8. Leave a stable artifact boundary that a future cross-Agent Eval Companion
   can import without depending on the skill's internal implementation.

## Non-goals

- Modifying Rust Agent Core, the `opi` CLI, Cargo manifests, or runtime
  semantics.
- Implementing Terminal-Bench, SWE-bench, AgentDojo, BFCL, or pi adapters.
- Creating the planned cross-Agent Eval Companion.
- Implementing containers, sandboxes, remote execution, or remote scheduling.
- Adding default telemetry, background upload, or automatic session capture.
- Implementing old/new Skill comparison; that remains a separate follow-up.
- Automatically remediating a failed case or modifying product source.
- Capturing, persisting, or grading private raw Chain-of-Thought.
- Making an LLM judge authoritative over deterministic environment outcomes.

## Considered approaches

### A. Deepen `opi-eval` in place

Keep the explicit skill entry point and add structured cases, a small
standard-library Python tool, calibration records, deterministic grading, and
offline reports.

Advantages:

- minimum change that closes the current evidence gap;
- preserves the explicit, credential-consuming invocation boundary;
- does not create a second general benchmark platform; and
- can later export stable cases and bundles to the independent Eval product.

Limitation: this remains a project-local workflow, not a general-purpose Eval
library or service.

### B. Build a standalone Python evaluation control plane now

Create a new repository-level runner with its own package, registry, and CLI.

Advantages: clearer standalone packaging and stronger immediate isolation.

Disadvantages: overlaps the planned cross-Agent Eval Companion, expands the
first delivery substantially, and risks freezing an interface before Opi has
validated it on a small suite.

### C. Specify schemas and process only

Document cases, rubrics, and calibration without executable validation or
grading.

Advantages: lowest implementation cost.

Disadvantages: preserves the present failure mode in which the intended
evidence system exists only in prose.

**Decision:** Use approach A. The local workflow proves the evidence contract;
it does not become the long-term cross-Agent evaluation product.

## Placement and ownership

The workflow remains owned by `.claude/skills/opi-eval/`:

```text
.claude/skills/opi-eval/
  SKILL.md
  cases/
    candy.toml
    context_retention.toml
    tool_chain.toml
  calibration/
    README.md
  scripts/
    fixtures/
      tool_chain_bundle/
        manifest.json
        trials/trial-001/
          output.ndjson
          capture-manifest.json
          captured/result.txt
    opi_eval.py
    test_opi_eval.py
  references/
    evaluator-prompt.md
    report-template.md
    test-cases.md
```

Runtime artifacts live outside version control:

```text
target/opi-eval/<run-id>/
  manifest.json
  cases/<case-id>.toml
  trials/<trial-id>/
    output.ndjson
    captured/
    capture-manifest.json
    grade-<grader-digest>.json
```

Small human-readable reports and append-only summary history remain under
`docs/eval/`. They contain artifact digests and bounded evidence summaries, not
raw prompts, tool results, full workspaces, or credentials.

No crate depends on this workflow. No evaluation behavior becomes an Agent
Core requirement.

## External interface

`.claude/skills/opi-eval/scripts/opi_eval.py` is the single executable entry
point. It uses only the Python standard library and exposes these subcommands:

```text
opi_eval.py validate [--case <id> ...]
opi_eval.py run [--case <id> ...] [--model <provider:model>] [--trials <n>]
opi_eval.py grade <run-bundle>
opi_eval.py evaluator-input <run-bundle>
opi_eval.py calibrate <gold-set> <machine-labels>
opi_eval.py report <run-bundle> [--machine-labels <path>] [--calibration <path>]
```

The script hides case parsing, evidence capture, grading, calibration, and
report construction behind these operations. Callers do not import its
internal functions as a stable library API.

`run` continues to consume real provider credentials and credits. `SKILL.md`
must not invoke it without an explicit user request; a human may still call
the script directly. `validate`, `grade`, `evaluator-input`, `calibrate`, and
fixture-backed tests are network-free.

The Python tool does not call an LLM judge. `evaluator-input` emits the bounded
evidence projection defined by the case rubrics. `SKILL.md` may dispatch the
existing readonly evaluator after deterministic grading, record its identity
and fingerprint, and pass the returned machine-label file back to `calibrate`
or `report`. The Python tool validates that file before using it.

The skill host, not the LLM, computes the fingerprint from the provider, model,
system-prompt digest, rubric digest, evidence-projection schema, and
output-parser schema. The LLM returns label content; the host wraps it with the
computed identity and fingerprint before validation. An echoed or invented
fingerprint from model output has no authority.

## Component responsibilities

### Case Registry

The Case Registry loads TOML manifests, validates schema and identifiers,
resolves the selected suite, and computes a canonical case digest. It never
builds Opi, executes a provider, or writes a verdict.

### Runner

The Runner applies the existing persistent Cargo-cache policy, verifies the
fresh release binary, freezes a resolved run manifest, creates one isolated
temporary workspace per trial, executes Opi, and captures bounded evidence.
It does not decide whether the observed behavior is correct.

### Grader

The Grader consumes only a sealed run bundle and case manifest. It performs
deterministic outcome and trajectory assertions and can therefore regrade
offline. A grade is a new immutable artifact keyed by the case, evidence,
grader, and rubric digests; it never mutates the run bundle.

### Calibrator and Reporter

The Calibrator compares versioned human gold labels with machine labels. The
Reporter exposes authoritative deterministic results, calibrated subjective
results, uncalibrated diagnostics, resource metrics, and attribution as
separate sections. Neither component changes a case or execution artifact.

## Case manifest

Each `cases/<id>.toml` is the authoritative source for one case. The initial
schema has this shape:

```toml
schema_version = 1
id = "tool_chain"
version = 1
suite = "local-smoke"
status = "pinned" # candidate | pinned | retired
risk_class = "workspace-write" # read-only | workspace-write
capture_paths = ["result.txt"]

[source]
kind = "manual" # manual | issue | eval-finding | session-export
reference = "git:c4994d4a177d9aee9c4713bef46fad2367e0cf1e:.claude/skills/opi-eval/references/test-cases.md#case-2-tool-chain"
sha256 = "a6c03e35c51214c5ba422bec33a81b88bc71cd825c965061d82151f346886095"

[review]
rubric_owner = "rubric-owner"
reviewers = ["reviewer-a", "reviewer-b"]
reviewed_at = "2026-08-11T00:00:00Z"
privacy_status = "approved"

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
prompt = """Read the file test-fixture.txt in the current directory, count the
number of lines it contains, then write a new file called result.txt containing
only the line count as a plain integer. Do not include any other text in
result.txt."""
user_value = "Reliably complete a bounded file transformation."
expected_behavior = "Read the input, count lines, and persist the count."

[[assertions]]
id = "outcome.result-text"
kind = "file_text_equals"
severity = "required"
path = "result.txt"
expected = "10"
allow_trailing_newline = true
```

Manifest validation enforces:

- the filename is exactly `<id>.toml`, and `id` is lowercase snake case;
- `schema_version` is `1` and `version` is a positive integer;
- `source.sha256` is the lowercase SHA-256 digest of the exact UTF-8 bytes in
  `task.prompt`; `source.reference` independently preserves the reviewed
  provenance anchor;
- `review` is required for `pinned` and `retired` cases and omitted for an
  unreviewed candidate;
- `retirement` is required only for a retired case and contains a non-empty
  reason, UTC retirement time, and an optional replacement case ID;
- reviewer and owner values are stable project-local identifiers, not email
  addresses or display names;
- `risk_class`, `tool_profile`, status, source kind, and privacy status are
  closed enums;
- `minimum_pass_rate` is in the inclusive range `0.0..1.0`;
- fixture and capture paths are normalized, workspace-relative paths without
  `..`, drive prefixes, or symlink traversal;
- fixture content is UTF-8 text and subject to a fixed per-file size limit;
- `task.prompt`, `task.user_value`, and `task.expected_behavior` are non-empty;
- every pinned case has at least one required outcome assertion; and
- every assertion and subjective rubric ID is unique inside the case.

The first schema does not support arbitrary setup or grader shell commands.
Those would make case review equivalent to approving executable code and would
duplicate the command-execution policy problem.

## Assertions and rubrics

The first schema supports only the assertions needed to cover the three
existing cases honestly:

- `exit_code_equals`;
- `final_text_regex`;
- `file_exists`;
- `file_text_equals`;
- `tool_call_sequence`;
- `tool_call_count`;
- `no_tool_error`; and
- `max_retry_count`.

Every assertion table has:

- a stable assertion ID;
- `required` or `advisory` severity;
- a supported `kind` whose fields determine one evidence source;
- bounded expected data; and
- a deterministic result of `1`, `0`, or `unknown`.

The initial kind-specific fields and semantics are:

| Kind | Fields | Evidence and comparison |
|---|---|---|
| `exit_code_equals` | integer `expected` | Exact process exit status. |
| `final_text_regex` | string `pattern` | Regex search over the extracted final assistant text. |
| `file_exists` | string `path`, boolean `expected` | Presence of an allowlisted captured regular file. |
| `file_text_equals` | string `path`, string `expected`, boolean `allow_trailing_newline` | Exact UTF-8 content, optionally after removing one terminal newline. |
| `tool_call_sequence` | string array `expected`, fixed `mode = "ordered-subsequence"` | Tool names extracted from NDJSON occur in the specified order; unrelated calls may occur between them. |
| `tool_call_count` | integer `minimum`, integer `maximum` | Inclusive count of tool execution starts. |
| `no_tool_error` | boolean `expected = true` | No extracted tool completion has `is_error = true`. |
| `max_retry_count` | integer `maximum` | Extracted auto-retry starts do not exceed the maximum. |

Missing valid evidence returns `unknown`. Malformed NDJSON, a schema violation,
or grader malfunction returns `error`; it must not be converted to `unknown`.

Subjective rubrics use the same stable IDs and result vocabulary. A subjective
rubric table contains `id`, positive integer `version`, `severity`, `question`,
and a non-empty allowlist of evidence projections such as `final_text`,
`tool_calls`, or `captured:<path>`. It must ask one atomic factual question. It
must not request a broad 1-10 score or combine correctness, style, efficiency,
and safety into one judgment.

Each subjective rubric also contains a `calibration` table with
`minimum_samples`, `minimum_human_human_exact`,
`minimum_human_machine_exact`, `maximum_unknown_rate`, and the allowed
`evaluator_fingerprint`. Threshold rates are in `0.0..1.0`; the fingerprint is
a lowercase SHA-256 digest.

Execution or grader malfunction is recorded separately as `error`; it is not a
fourth rubric label.

## Run bundle and evidence capture

Before execution, the Runner freezes a resolved manifest containing:

- run and trial IDs;
- case ID, version, and digest;
- a byte-for-byte copy of each reviewed case manifest, with its artifact
  digest, so deterministic grading does not depend on a mutable live registry;
- Git commit and dirty-worktree disclosure;
- release binary path and SHA-256;
- provider/model identity and the resolved control fingerprint available to
  the current CLI;
- tool policy, timeout, trial count, and resource limits;
- operating-system and architecture metadata;
- start time; and
- artifact schema version.

Each trial receives a fresh workspace. The Runner captures:

- stdout NDJSON and process exit status;
- wall time;
- bounded session-summary usage and diagnostics;
- only the final files listed in `capture_paths`;
- a capture manifest containing path, size, media classification, and digest;
  and
- redaction and leakage-scan results.

Capture paths must resolve inside the trial workspace. Captured files must be
regular files, use an allowed text type in the first schema, and remain below a
fixed size bound. The Runner never copies the whole workspace.

After redaction and validation, the bundle is sealed. A re-run always receives
a new trial ID. A regrade writes a new grade artifact and never edits prior
evidence. The normalized grade payload excludes volatile generation times so
the same evidence and grader version produce byte-stable normalized output;
an outer artifact envelope may record when that grade was generated.

## Execution and grading flow

```text
validate case
  -> build and hash fresh release binary
  -> resolve model, config, tools, and limits
  -> freeze run manifest
  -> execute each trial in a fresh workspace
  -> capture NDJSON and allowlisted final state
  -> redact, validate, digest, and seal evidence
  -> run deterministic offline graders
  -> optionally run the subjective evaluator
  -> check calibration authority
  -> render report and append history summary
```

No automatic retries until success are allowed. A replacement execution is a
new trial and remains linked to the failed trial in the report.

## Verdict model

Per-rubric values are `1`, `0`, and `unknown`. Case-level verdicts are:

| Verdict | Meaning |
|---|---|
| `PASS` | Every required and advisory assertion passed. |
| `DEGRADED` | Every required assertion passed and at least one advisory assertion failed. |
| `FAIL` | At least one required assertion failed. |
| `INCONCLUSIVE` | No required assertion failed, but at least one required assertion is unknown. |
| `ERROR` | Execution, evidence sealing, or grading could not complete. |

Authority is ordered and non-compensatory:

1. deterministic outcome assertions;
2. deterministic trajectory assertions;
3. calibrated subjective rubrics; and
4. uncalibrated LLM diagnostics.

A lower authority cannot override a higher authority. In particular, an LLM
cannot turn an incorrect final file, failed deterministic assertion, or
observed authority violation into a passing result.

Every trial is graded separately. Reports publish the number of passing trials
and the pre-registered minimum pass rate; they never select a best trial.

## Efficiency baselines

The first run of a case does not implicitly become a baseline. An efficiency
baseline must be an explicitly promoted, reviewed run whose case, model,
control fingerprint, tool policy, and environment class match the candidate
comparison.

Without such a baseline, resource rubrics remain `unknown` and appear only as
observed measurements. They cannot produce a passing efficiency claim.

## Human and machine calibration

Human gold labels bind all of the following:

- case and case version;
- rubric ID and rubric version;
- evidence digest;
- final adjudicated label; and
- reviewer and Rubric Owner identities.

At least two human reviewers label the evidence independently. The designated
Rubric Owner adjudicates disagreements and owns the final versioned standard.
This role establishes one coherent contract; it does not permit silent or
retroactive rubric changes.

Each subjective rubric pre-registers:

- minimum calibration sample size;
- minimum human/human exact agreement;
- minimum human/machine exact agreement;
- maximum accepted `unknown` rate; and
- the evaluator fingerprint to which the calibration applies.

Reports always include exact agreement. They may also include Cohen's kappa
when the class distribution supports a meaningful calculation, but no single
aggregate statistic hides item-level disagreement.

Calibration is invalidated when any of these change:

- evaluator provider or model;
- evaluator system prompt;
- rubric text or version;
- evidence projection supplied to the evaluator; or
- output parser and label protocol.

Until calibration passes, LLM labels are diagnostic only and cannot affect the
authoritative verdict. After calibration, machine labels may satisfy subjective
rubrics but still cannot override deterministic outcomes.

## Case lifecycle

The lifecycle is:

```text
candidate -> pinned -> retired
```

### Candidate

A candidate can be written manually or imported explicitly from an issue,
normalized eval finding, or user-provided redacted session export. It is not
part of default regression. AI may help draft a candidate but cannot promote
it.

### Pinned

A human may promote a candidate only after all of these pass:

- schema and stable-ID validation;
- source digest and duplicate detection;
- secret, absolute-path, and configured-sensitive-value scan;
- at least one required deterministic outcome assertion;
- isolated reproduction evidence; and
- explicit Rubric Owner approval.

Only pinned cases run in the default `local-smoke` suite.

### Retired

A retired case remains in the registry with its ID, reason, retirement date,
and replacement case when one exists. It is excluded from execution but
preserves report interpretability.

Lifecycle transitions never occur as a side effect of `run`, `grade`, or
`report`. In the first version, promotion and retirement are reviewed manifest
edits in Git; the tool validates transition metadata but does not perform the
transition automatically.

## Privacy and safety

- The workflow adds no telemetry, remote storage, or background upload.
- Session-derived cases require explicit user export and redaction.
- Case fixtures and captured paths are bounded and workspace-relative.
- A failed secret, path, or configured-sensitive-value scan blocks bundle
  sealing and any write to `docs/eval/`. The first version supports known
  secret patterns and an explicit denylist supplied by the case or user; it
  does not claim generic personal-data detection.
- Raw evidence stays under local `target/opi-eval/` or an explicitly selected
  private artifact store.
- Git reports contain only bounded summaries, stable references, and digests.
- The evaluation script does not implement a sandbox. It invokes Opi with the
  case's reviewed tool profile and preserves the existing product safety
  boundaries.
- No private raw reasoning is requested or stored. Trajectory evidence is
  limited to externally observable messages, tool calls/results, retries,
  compaction events, diagnostics, and final artifacts.

## Reporting and attribution

Reports connect four levels without collapsing them into one score:

```text
user value
  -> task outcome
  -> Agent behavior
  -> runtime signals
```

For example, a coding task may map issue resolution to test/file outcomes,
then to tool selection and recovery behavior, then to token, latency, retry,
and compaction measurements.

The report separates:

- authoritative deterministic outcomes;
- deterministic trajectory findings;
- calibrated subjective findings;
- uncalibrated diagnostics;
- resource measurements and baseline coverage;
- evidence completeness; and
- attribution.

Attribution uses this fixed initial taxonomy:

- `agent`;
- `prompt-or-skill`;
- `tool`;
- `runtime`;
- `model-or-provider`;
- `environment`;
- `grader`; and
- `infrastructure`.

The report distinguishes directly observed facts from inferred attribution.
An LLM explanation is an inference until a deterministic reproduction or
human review confirms it.

## Error behavior

- Invalid cases fail validation before build or provider use.
- A failed fresh build stops execution; a stale binary is never used.
- Agent crashes and timeouts are trial outcomes and remain visible.
- Infrastructure and grader failures use their own classifications and are not
  converted to Agent failures or silent skips.
- An incomplete required evidence source produces `unknown` and therefore an
  `INCONCLUSIVE` result unless another required assertion already failed.
- Evidence-sealing or grader malfunction produces `ERROR`, while preserving
  safe evidence needed for diagnosis.
- Redaction or leakage failure blocks report/history publication.
- Regrading failure does not mutate the sealed run bundle or an older grade.

## File impact

Implementation is expected to modify or add only:

- `.claude/skills/opi-eval/SKILL.md`;
- `.claude/skills/opi-eval/cases/*.toml`;
- `.claude/skills/opi-eval/scripts/opi_eval.py`;
- `.claude/skills/opi-eval/scripts/test_opi_eval.py`;
- `.claude/skills/opi-eval/scripts/fixtures/tool_chain_bundle/**`;
- `.claude/skills/opi-eval/calibration/README.md`;
- `.claude/skills/opi-eval/references/test-cases.md`;
- `.claude/skills/opi-eval/references/evaluator-prompt.md`;
- `.claude/skills/opi-eval/references/report-template.md`; and
- `docs/eval/README.md`.

`references/test-cases.md` becomes an authoring guide and index whose
authoritative case definitions are the TOML manifests. It must not duplicate
full prompts and assertion definitions.

No Rust, Cargo, implementation-ledger, release-state, changelog, or normative
`docs/opi-spec.md` change is part of this delivery.

## Testing

Tests use Python's standard-library `unittest`, isolated temporary directories,
saved NDJSON fixtures, and fake executables. They never call a provider or
require credentials.

Required coverage:

1. valid and invalid case manifests;
2. every initial deterministic assertion;
3. `PASS`, `DEGRADED`, `FAIL`, `INCONCLUSIVE`, and `ERROR` aggregation;
4. workspace-relative capture, media/size bounds, and path rejection;
5. secret, absolute-path, and configured-sensitive-value leakage fixtures;
6. source digest, duplicate detection, and lifecycle transition validation;
7. exact-agreement calculation and evaluator-fingerprint invalidation;
8. proof that an uncalibrated LLM result cannot alter the authoritative
   verdict;
9. deterministic regrading from a saved bundle;
10. equivalent migration of the existing `candy`, `tool_chain`, and
    `context_retention` cases; and
11. proof that `tool_chain` inspects final `result.txt` rather than passing from
    tool events alone.

Verification commands:

```text
python -m unittest .claude/skills/opi-eval/scripts/test_opi_eval.py
python .claude/skills/opi-eval/scripts/opi_eval.py validate
python .claude/skills/opi-eval/scripts/opi_eval.py grade <saved-fixture-bundle>
python scripts/opi-doc-check.py
git diff --check
```

No Rust compile is required because this delivery affects skill behavior,
Python tooling, manifests, and documentation only. Test impact is `add` for the
new Python suite and `update` for the three migrated eval cases.

## Acceptance criteria

1. The three existing local-smoke cases are driven by validated TOML manifests.
2. Every pinned case has at least one required deterministic outcome assertion.
3. `tool_chain` passes only when the captured final `result.txt` contains the
   expected value.
4. A sealed bundle can be regraded offline with byte-stable normalized output
   for the same grader version.
5. Missing required evidence remains `unknown` and yields `INCONCLUSIVE` rather
   than zero, pass, or silent omission.
6. An uncalibrated evaluator cannot affect the authoritative verdict.
7. Changing the evaluator or rubric fingerprint invalidates prior calibration.
8. A candidate cannot enter default regression without explicit review and
   promotion.
9. A leakage failure prevents raw evidence or unsafe summaries from entering
   `docs/eval/`.
10. Reports visibly separate outcome, trajectory, diagnostic, efficiency,
    evidence-coverage, and attribution claims.
11. All tests and documentation checks pass without a provider credential or
    network access.
12. The Case, run-bundle, and grade schemas are versioned and documented well
    enough for a later Companion importer without exposing script internals as
    a stable API.

## Follow-up boundaries

After this design is implemented and exercised, handle these as separate
designs:

1. old/new behavioral evaluation of project-local Skills against pinned tasks;
2. the independent cross-Agent Eval Companion and its Agent/Grader adapters;
3. Terminal-Bench and SWE-bench integrations;
4. production-scale case management or hosted artifact storage; and
5. benchmark release and promotion gates.

None of those follow-ups may weaken deterministic outcome authority, evidence
provenance, explicit user consent, or the parent specification's separation
between Eval, Agent Core, sandbox ownership, and promotion authority.
