# Active Audit Set Contract

Each Phase has one live assurance set under `docs/snapshots/phase<N>/assurance/`.
The set contains one independently installed report group per reviewer/model
pair and no canonical report:

```text
docs/snapshots/phase<N>/assurance/
├── audit.index.json
├── audit.codex.gpt56.meta.json
├── audit.codex.gpt56.requirements.jsonl
├── audit.codex.gpt56.findings.jsonl
├── audit.codex.gpt56.md
├── audit.claude.glm53.meta.json
├── audit.claude.glm53.requirements.jsonl
├── audit.claude.glm53.findings.jsonl
├── audit.claude.glm53.md
└── history/<audit-run-id>/...
```

The general stem is `audit.<reviewer-id>.<model-id>`. Both IDs match
`^[a-z0-9][a-z0-9-]*$`, and the pair is unique in the live set. `<model-id>`
discloses the model selected by the runtime; it does not configure or select
a provider model. There is no unsuffixed compatibility alias.

`audit.index.json` is the only active-membership authority. An active-style
file absent from the index is an orphan and makes the set invalid. Content in
`history/` is immutable evidence and is never an independent-core, verdict or
remediation source input. The post-install advisory exception below permits
bounded historical comparison and exact raw-proof lookup without changing those
authority boundaries.

## Independent input boundary

`phase=<N>` locates the committed implementation ledger, pointed Phase state,
and registered sources. Each peer derives its baseline independently from the
latest committed `docs/opi-spec.md` and every currently registered
supplemental source; stored historical hashes are mismatch evidence only.
Members of the live set may have been sealed at different `audit_head`s.
Every member's head must be a committed ancestor of the current HEAD, checked
with `git merge-base --is-ancestor <audit_head> HEAD`; a one-member live set
is valid and consumable at any time, and no consumer may assume a roster.

Run `rotation` before requirements are read. It inspects Git path state and
names only. Before successful `complete`, no auditor may read, search, summarize,
or compare a prior audit, remediation conclusion, history run, or another peer's
staged output. Each peer derives requirements and inspects a sealed source export
in an independent context. Historical exposure before installation invalidates
that peer. Exposure after installation cannot authorize revision of its core;
only the advisory compiler described below may proceed in that context.

Root and Phase ledgers can embed earlier conclusions. The
[audit workflow's scope reader](../../opi-audit/SKILL.md#2-seal-this-peers-baseline)
projects allowed fields before they reach the auditor; baseline digests still
cover the complete committed bytes. Ledger files are not direct context input.

## Active index

The index uses schema version 2:

```json
{
  "schema_version": 2,
  "phase": 17,
  "revision": 3,
  "aggregate_verdict": "FAIL",
  "members": [
    {
      "reviewer_id": "codex",
      "model_id": "gpt56",
      "artifact_stem": "audit.codex.gpt56",
      "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z",
      "audit_head": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
      "verdict": "FAIL",
      "digests": {
        "meta_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "requirements_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "findings_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "report_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
      }
    }
  ]
}
```

Members are sorted by `(reviewer_id, model_id)`. Every member has exactly four
files, one unique `audit_run_id`, its own full lowercase-SHA `audit_head`, and
exact raw-byte SHA-256 digests. Filename, index, metadata, and report headers
agree on Phase, reviewer/model identity, run, head, and verdict. All members
share the same `baseline_policy` and the same ordered `baseline_sources` path
list; the per-path digests may differ because members were sealed at different
heads. The aggregate verdict is fail-dominant:

```text
FAIL                if any member is FAIL
PASS-WITH-FINDINGS  if none fail and any member is PASS-WITH-FINDINGS
PASS                if every member is PASS
```

Structural, identity, digest, membership, or recovery errors yield
`AUDIT-INCOMPLETE`; they never yield a verdict.

## Member metadata and records

Each `audit.<reviewer-id>.<model-id>.meta.json` uses schema version 3:

```json
{
  "schema_version": 3,
  "audit_run_id": "phase17-codex-gpt56-136c380-20260824t010203z",
  "phase": 17,
  "audit_head": "136c380f0c5eea541190cc1a0f5c1d62f983b4e8",
  "reviewer_id": "codex",
  "reviewer_identity": "Codex",
  "model_id": "gpt56",
  "reviewer_model_id": "gpt-5.6",
  "model_identity_source": "runtime-attested",
  "independence": "fresh-context-same-family",
  "baseline_policy": "latest-committed-spec",
  "baseline_sources": [
    {"path": ".opi-impl-state.json", "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
    {"path": "docs/snapshots/phase17/opi-impl-state.json", "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
    {"path": "docs/opi-spec.md", "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"}
  ],
  "requirements_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
  "findings_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "verdict": "FAIL"
}
```

`model_identity_source` is `runtime-attested`, `request-config`, or
`operator-declared`. The workflow never infers an exact model from a family or
self-description. Both human-readable identity fields are non-empty.

The requirements sidecar contains one JSONL record per independently decidable
requirement, sealed before production inspection. State is `met`,
`partially-met`, `not-met`, or `not-assessable`; missing mandatory evidence is
`not-assessable`. Every non-met mandatory requirement has a reciprocal finding.
Finding shape and identity are owned by `finding-contract.md`.

The member verdict is mechanical:

```text
FAIL                if any mandatory state is not met
PASS-WITH-FINDINGS  if all mandatory states are met and a non-Info finding exists
PASS                if all mandatory states are met and no non-Info finding exists
```

## Independent installation

No roster is pre-registered and no peer waits for another. Each peer renders
its four files in a private member directory outside the live assurance
directory and installs itself:

```text
python .agents/skills/_shared/scripts/assurance_set.py complete docs/snapshots/phase<N> <member-directory> --reviewer <reviewer-id> --model <model-id>
python .agents/skills/_shared/scripts/assurance_set.py recover docs/snapshots/phase<N>
python .agents/skills/_shared/scripts/assurance_set.py migrate docs/snapshots/phase<N>
```

`complete` holds the repository lock `opi-assurance-locks/phase<N>.lock`. It
validates the staged member (exactly the four suffixed files, LF-only bytes,
self-describing metadata), requires rotation admission — the live assurance
paths must be tracked and clean — requires the member's `audit_head` to be a
committed ancestor of HEAD, and requires the current live set to validate
before any mutation. A staged report containing the reserved
`<!-- opi-audit-history/` marker is rejected: independent installation must precede
historical compilation. It then installs the member files and writes
`audit.index.json` last as the membership switch. `revision` advances by one
per accepted install.

Installing the same `(reviewer_id, model_id)` again replaces that member's
own entry: the superseded run's four files move to
`assurance/history/<audit-run-id>/` and no other member is touched.
Re-installing the identical `audit_run_id` is refused. Any membership change
changes the index bytes and thereby invalidates an existing remediation plan
through its index-digest binding.

Installations are journaled under `opi-assurance-transactions/` with states
`prepared`, `installing`, and `switched`. Before the switch, recovery restores
the exact prior bytes and removes only transaction-owned files; after a
validated switch, recovery completes the history move. Every consumer
recovers under the same lock before reading live membership. `migrate` is the
one-time conversion of a legacy schema-1 generation set into the live
schema-2 set: it is format-only — metadata drops the generation identity
field, reports drop the superseded generation header line, requirements and
findings sidecars stay byte-identical, and the digests and index are rebuilt.
A pre-existing history target, admission failure, or validation failure
leaves the prior set active and reports `AUDIT-INCOMPLETE`.

## Post-audit historical reference

Successful `complete` freezes the independent core: the original report bytes,
metadata, requirements, findings and verdict. Only afterward may the compiler
read relevant same-Phase reports and repairs pinned to committed versions
available at `audit_head`. It does not ingest concurrent sibling output or wait
for a roster. Current behavior remains anchored to `audit_head`. Discoveries
after historical exposure are leads for fresh independent verification, never
edits or additions to the installed core.
Record any newly noticed post-install defect absent from the current sidecar
only as a bounded lead in comparison `limitations` for fresh independent
verification; do not invent a current finding ID or an old-only comparison row.

Normal workflow completion requires an installed `compared` or bounded
`no-history` supplement. If comparison cannot finish, preserve the valid core and
report `HISTORY-SUPPLEMENT-INCOMPLETE` with the missing evidence or failure. This
does not change its verdict or invalidate the intermediate core. Legacy reports
without appendices remain valid and fully consumable. Fresh independent auditors
exclude prior complete reports, including their appendices.

### Comparison input

Supply UTF-8 JSON with exactly these fields; all descriptive strings are
non-empty. Hashes are lowercase SHA-256 and commits are full lowercase 40-hex
identities. There is no fifth member file or historical findings ledger.

| Field | Shape / meaning |
|---|---|
| `schema` | `opi-audit-history/1` |
| `status` | `compared` or `no-history` |
| `scope` | Inspected same-Phase history and the boundary of any absence claim |
| `sources` | List of `{id, path, commit, sha256, audit_run_id}`; unique source IDs, exact committed source bytes, and owning historical run or `null` for a non-audit source |
| `comparisons` | Comparison rows defined below; empty for `no-history` |
| `limitations` | Missing inputs, applicability limits, or an explicit statement that none were found within scope |

Each comparison has exactly these fields:

| Field | Shape / meaning |
|---|---|
| `current_finding_id` | Finding in this member's current sidecar, or `null` for an old-only preserved repair; the envelope supplies its owning run and findings digest |
| `prior_findings` | List of `{source_id, finding_id, audit_run_id}`; each source ID resolves to the pinned historical source with that run |
| `relationship` | `persistent`, `missed-branch`, `regression`, `new-omission`, `evidence-gap`, `insufficient-information`, or `preserved-repair` |
| `prior_claim` | Attributed historical claim or repair promise; do not adopt old `Closed` as current closure |
| `repair_commits` | List of full commit identities; empty when none is established |
| `current_evidence` | Current `audit_head` code/check evidence and what remains uncertain |
| `cause` | Supported explanation or explicitly bounded inference |
| `raw_evidence` | List of `{path, sha256, checked, limitations}`; exact candidate raw proof, checks actually performed, and remaining applicability limits |
| `verification` | Current checks or applicability proof suggested for remediation |

`persistent` means the same defect remains; `missed-branch` identifies a different
path violating the same invariant. `regression` requires evidence of a working
repaired state and a later failing state. `evidence-gap` distinguishes unassessed
proof from a reproduced failure; `preserved-repair` records a verified retained
fix. Use `insufficient-information` rather than infer recurrence from matching
titles. `no-history` asserts only the inspected scope; unavailable inputs are a
limitation, not proof of absence. Historical-only rows remain outside repair scope.

### Envelope and locked append

Run after independently installing the member:

```text
python .agents/skills/_shared/scripts/assurance_set.py append-history docs/snapshots/phase<N> <comparison-json> --reviewer <id> --model <id> --audit-run-id <run> --expected-index-sha256 <digest>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```

The tool derives the envelope from the installed report. Its fields are
`schema`, `audit_run_id`, `audit_head`, `core`, and `comparison`. `core` contains
`report_bytes`, `report_sha256`, `meta_sha256`, `requirements_sha256` and
`findings_sha256`; `comparison` contains the input above. The reserved envelope
marker is `<!-- opi-audit-history/`. The tool renders the terminal section
**Post-audit historical reference — advisory**, escaping record text. Do not
construct or edit this suffix by hand. Validation checks its canonical rendering,
the exact prefix length/digest, owning identities, sidecar digests and current
finding references; it does not establish the truth of historical prose.

Under the existing Phase lock, recover interrupted transactions, validate the
live set and exact target run, and check `--expected-index-sha256` before any
write. Only the target report and live index may change. Preserve the complete
core prefix, all three sidecars, every other member, metadata independence class
and all verdicts. The report digest changes and index revision advances once.
The existing index schema 2 and metadata schema 3 stay unchanged.

This operation does not require rotation's clean-worktree gate: a freshly
installed core is normally uncommitted. Exact validated bytes and index identity
admit the append without an intermediate commit. Arbitrary report/index edits
remain invalid. Before a validated transaction switch, recovery restores exact
prior report/index bytes; after it, recovery retains the complete append. No
immutable history directory is rewritten.

An identical installed comparison is a no-op after live-set and exact-target
validation, including a retry with its original pre-append index digest. Every
operation that would write requires the current expected digest. A different
second appendix is rejected; no replacement/edit-history operation is admitted.

### Remediation use

After validating the current index and strict finding union, remediation may
read an indexed report's validated appendix for candidate code paths, regression
cases and raw proofs. It cannot confirm, refute or close a finding, lower
severity, add or remove source keys, or determine closure clustering. Historical
`lineage`, `prior_occurrences` and `prior_disposition` fields remain forbidden.
Do not search history to reconstruct a missing appendix.

The only raw-proof exception permits reading exact proofs named by the appendix.
Before using them at `remediation_head`, independently verify bytes/digests,
commands/results and executed counts, source/build/client/material identities,
and the predicate proved. Prove relevant dependencies unchanged or rerun
invalidated checks within existing authorization. Missing evidence remains
unconfirmed; historical success and matching hashes alone are insufficient.
Live calls, credentials and paid runs retain their existing authorization limits.

The full report remains bound by `report_sha256`, so appending changes the index
and invalidates an already approved remediation plan. Complete available
enrichment before planning; never waive the binding for an advisory-only change.

## Durability

Assurance digests are exact raw bytes, so artifacts must be byte-stable
across checkouts. The repository pins
`docs/snapshots/**/assurance/** text eol=lf` in `.gitattributes`; staged
member files containing any CR byte are rejected at `complete`; normalize a
legacy working tree to LF before migrating it.

Validate with:

```text
python .agents/skills/_shared/scripts/validate_assurance_artifact.py rotation docs/snapshots/phase<N>
python .agents/skills/_shared/scripts/validate_assurance_artifact.py audit-set docs/snapshots/phase<N>/assurance
```

Rotation requires the current assurance paths to be tracked and clean and
rejects legacy Phase-level siblings without reading conclusions. Direct file
copying, glob-discovered membership, vote-based aggregation, unsuffixed
fallback, and prior-conclusion input to the independent core are forbidden.
The post-install advisory exception does not weaken these installation rules.
