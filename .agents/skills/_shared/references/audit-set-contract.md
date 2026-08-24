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
`history/` is immutable evidence and is never an audit, verdict, or remediation
input.

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
names only. No auditor may read, search, summarize, or compare a prior audit,
remediation conclusion, history run, or another peer's staged output. Each
peer derives requirements and inspects a sealed source export in an
independent context. Exposure to a sibling conclusion invalidates that peer.

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
before any mutation. It then installs the member files and writes
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
fallback, and prior-conclusion input are forbidden.
