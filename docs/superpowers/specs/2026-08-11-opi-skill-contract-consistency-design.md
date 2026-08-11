# Opi Skill Contract Consistency Design

**Status:** Approved design

**Date:** 2026-08-11

**Scope:** Mechanical consistency checks for project-local `.claude/skills`
contracts and their Codex sidecars. Minimum-change admission, audit semantics,
and behavioral skill benchmarking are separate follow-up designs.

## Problem

Opi's ten project-local skills intentionally require explicit invocation on
both Claude and Codex. The authoritative workflow remains in each skill's
`SKILL.md`, while `agents/openai.yaml` provides a thin Codex-facing adapter and
the English/Chinese indexes describe the collection.

Those copies can drift. Two current examples are:

- `opi-audit/agents/openai.yaml` describes an "implementation range", while
  `opi-audit/SKILL.md` defines current committed `HEAD` as the sole audit
  endpoint and forbids commit-range coverage boundaries;
- `opi-eval/agents/openai.yaml` asks for a "requested phase", while the skill
  accepts `model` and `cases`, not a phase.

`scripts/opi-doc-check.py` already validates source-derived documentation
contracts, but it does not inspect `.claude/skills` or Codex sidecars.

## Goals

1. Detect mechanically provable drift between skill directories, skill
   frontmatter, Codex sidecars, and the EN/ZH skill indexes.
2. Correct the two known sidecar mismatches.
3. Keep the check fast, network-free, dependency-free, and part of the existing
   documentation gate.
4. Produce precise path-oriented failures that can be fixed without running a
   Rust build.

## Non-goals

- Inferring full semantic equivalence between free-form skill prose and a
  sidecar prompt.
- Generating sidecars or introducing a new skill manifest.
- Adding another workflow, task ledger, invocation mode, or persistent state.
- Implementing the minimum-change admission lens, complexity audit lens, or a
  model-based behavioral benchmark in this change.
- Reformatting or otherwise rewriting unrelated skill content.

## Considered approaches

### A. Extend `opi-doc-check.py` with source-derived structural checks

Discover `.claude/skills/opi-*` directories, parse the narrow frontmatter and
sidecar fields needed by the contract, and compare their derived sets with the
two workflow indexes.

Advantages: reuses the existing CI gate, adds no dependency or new command,
and keeps the check proportional to a documentation/skill-only change.
Limitation: it cannot prove that arbitrary prose in a default prompt is
semantically identical to the full skill.

### B. Add a canonical skill manifest and generate all sidecars

This gives the strongest single-source model, but it adds a manifest,
generation command, migration, and another artifact that every skill edit must
understand. Ten small sidecars do not yet justify that machinery.

### C. Add a separate `opi-skill-check.py`

This separates concerns, but creates a second CI/documentation command and a
new place whose invocation must remain synchronized with `opi-document`, CI,
and contributor guidance.

**Decision:** Use approach A. If structural checks repeatedly fail to prevent
semantic drift, that is evidence for approach B; do not pre-build it now.

## Contract checked

For every immediate child directory matching `.claude/skills/opi-*`:

1. Exactly one case-preserving skill entry file exists: `SKILL.md` or the
   repository's existing lowercase `skill.md` form. Discovery enumerates the
   directory and compares names case-insensitively so Windows does not count
   one file twice through two differently cased candidate paths.
2. Its YAML-like frontmatter is present, its `name` equals the directory name,
   and `disable-model-invocation` is exactly `true`.
3. `agents/openai.yaml` exists.
4. The sidecar has a non-empty `interface.display_name`,
   `interface.short_description`, and `interface.default_prompt`.
5. The default prompt invokes the same skill using `$<directory-name>`.
6. `policy.allow_implicit_invocation` is exactly `false`.
7. The set of discovered skill names equals the skill-index table in both
   `.claude/skills/README.md` and `.claude/skills/README.zh.md`.
8. Existing local-link validation includes the two skill indexes and every
   selected `SKILL.md` file.

The parser is intentionally narrow. It reads only simple top-level frontmatter
keys plus the two known sidecar sections; unsupported or ambiguous structure
fails with the owning path instead of silently guessing.

Full prompt semantics remain human-reviewed. The implementation corrects the
two known mismatches, but does not claim that a structural parser can prevent
every future prose contradiction.

## Implementation shape

`scripts/opi-doc-check.py` gains small pure helpers for:

- discovering skill entry files;
- parsing the required scalar frontmatter fields;
- parsing the required Codex sidecar fields;
- extracting the skill-name set from each index table;
- appending contract failures to the existing `ERRORS` collection.

The new check is called once from `main()`. It returns the selected skill and
index paths so the existing local-link check can cover them without a second
filesystem walk.

The two sidecar prompts are corrected surgically:

- `opi-audit`: current committed `HEAD`, not an implementation range;
- `opi-eval`: selected runtime cases/model, not a phase.

`opi-document/SKILL.md` and its documentation-check reference are updated only
where they enumerate what `opi-doc-check.py` validates.

## Error behavior

- Missing, duplicate, malformed, or mismatched files add deterministic `FAIL:`
  entries and produce exit code 1 through the existing path.
- One malformed skill does not stop discovery of the remaining skills.
- No file is modified by the checker.
- No YAML dependency is added; the accepted subset is deliberately smaller
  than general YAML.

## Testing

Add `scripts/test_opi_doc_check.py` using Python's standard-library
`unittest`. Tests construct isolated temporary skill trees and replace the
module's `ROOT`/`ERRORS` for each case.

Required red/green cases:

1. a complete matching skill contract passes;
2. a frontmatter name/directory mismatch fails;
3. missing explicit Claude invocation metadata fails;
4. an implicit Codex sidecar fails;
5. a default prompt naming a different skill fails;
6. an EN or ZH index missing a discovered skill fails;
7. either existing `SKILL.md` casing convention is accepted.

Verification commands:

```text
python -m unittest scripts/test_opi_doc_check.py
python scripts/opi-doc-check.py
git diff --check
```

No Rust compile is required because the change affects Python documentation
checks, skill metadata, and prose only.

## Follow-up boundaries

After this subproject is verified, handle the remaining agreed ideas
separately:

1. minimum-change trace in `opi-implement plan` and task planning;
2. a complexity/minimality sub-lens under the `opi-audit` Standards axis;
3. isolated behavioral evaluation of skill changes against pinned tasks.

None of those follow-ups may create a second implementation ledger or weaken
registered specifications, acceptance scenarios, safety checks, or
verification tiers.

## Acceptance criteria

1. The checker discovers every project-local `opi-*` skill without a hard-coded
   count or name list.
2. All discovered skills have consistent directory names, frontmatter names,
   explicit Claude invocation metadata, and explicit Codex invocation policy.
3. Both skill indexes enumerate exactly the discovered set.
4. The known audit/eval sidecar mismatches are corrected.
5. New unit tests demonstrate failure before implementation and pass afterward.
6. The existing full documentation check and `git diff --check` pass.
7. No Rust source, canonical ledger, product spec, release state, commit, or
   external system is modified.
