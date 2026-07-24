---
name: opi-document
description: >-
  Update opi documentation so it stays truthful to the shipped code and the
  English/Chinese mirrors stay in sync. Use this skill whenever the user
  mentions "update the docs/README", "refresh the README", "sync EN/ZH",
  "translate the README", "fix doc drift", "文档更新", "文档同步", "更新 README",
  "翻译 README", or asks to bring docs in line after a change. Also use at
  Phase 6 of the opi workflow (after implementation/remediation, before
  release), and whenever a version bump needs the doc version-strings resynced.
---

# Opi Document

Keep opi documentation **truthful** — every claim matches the shipped source —
and keep the English and Simplified-Chinese mirrors in lockstep, with the
doc-guard tests **green**. This is Phase 6 of the opi workflow.

opi docs are not free prose. Eight guard-test suites read them from disk and
assert specific tokens (provider lists, the `opi-agent` API-surface table,
phase non-goal lists, exact tool-policy strings). Editing a doc without
preserving those tokens breaks `cargo test`. This skill edits **inside** the
guards, never around them.

## Inputs

```text
scope=<full | targeted | version-bump>   # default: targeted to changed surfaces
files=<path,...>                          # optional; explicit doc files
version=<X.Y.Z>                          # required for version-bump scope
```

If the user says "update the docs after the Phase 13 work" or "refresh the
README", infer scope from what changed. A version bump requires `version=`.

## Workflow

### Phase A — Discover the doc delta

Identify what shipped since the docs were last touched: read `CHANGELOG.md`
(`[Unreleased]` + recent releases), `git log` since the last doc-touching
commit, and the current `Cargo.toml` workspace version.

**Done when:** you hold a list of (doc file → facts that may have drifted) and,
if a version bump is in scope, the set of version-bearing lines (root `README` +
`AGENTS.md` + `CLAUDE.md` + the four crate `README`s, EN + ZH).

### Phase B — Audit current docs against source

For each affected doc, verify every checkable claim against authoritative source
(`Cargo.toml`, crate `src/`, `docs/opi-spec.md`, `CLAUDE.md`). Dispatch parallel
subagents for wide reads. Flag three classes:

- **drift** — wrong / outdated / unverifiable / missing-shipped claims
  (versions, provider ids, CLI flags, schema constants, limits, module lists).
- **noise** — internal milestone jargon a reader cannot act on ("Phase 11/12/13",
  "Workstream 10.x", spec-hash ledger references, "checkout may contain
  unreleased Phase N").
- **gaps** — what an adopter needs but is missing (one-line value prop, install
  line, runnable example).

**Done when:** every drift item cites the authoritative source that disproves
it, and every noise/gap item is listed with `file:line`.

### Phase C — Load the guard constraints

Read [`references/doc-guards.md`](references/doc-guards.md). For each file you
will touch, record the pinned tokens that must survive verbatim and the
forbidden overclaim phrases that may appear only negated.

**Done when:** for every target file, the pinned-token set and the
forbidden-phrase set that bound the edit are written down.

### Phase D — Decide guard-safe scope

Produce a per-file change plan: fix every drift, remove noise only where it is
**unpinned**, fill gaps. Never drop a pinned token; never introduce a forbidden
positive claim. If a requested style (e.g. a pi-style rewrite) collides with
guard-locked content, surface the conflict and pick the guard-safe option
**before** editing.

**Done when:** the plan preserves every pinned token (named per file) and adds
no forbidden phrase — stated explicitly, not assumed.

### Phase E — Edit English docs

Apply the plan. Preserve section headings and tables that guards parse —
notably the `opi-agent` `## API Surface Classification` … `## Non-Goals` block,
which `runtime_contract_docs.rs` parses against the `pub use`s in
`crates/opi-agent/src/lib.rs` (every crate-root re-export must stay a classified
table row; only the Notes cell is free text).

**Done when:** every drift item is resolved; no pinned token removed; no
forbidden phrase introduced outside a negation; version-bearing lines updated in
lockstep when a version bump is in scope.

### Phase F — Mirror to Chinese

Apply the **same surgical edits** to each `*.zh.md`, preserving every pinned ZH
token verbatim (`敏感`, `脱敏`, `任意流式 chunk`, `会话费用汇总会被省略`,
`OpenAI Chat 会从任何携带 \`id\` 的 chunk`, `捕获 response ID`, … — full list in
[`references/doc-guards.md`](references/doc-guards.md)). For **net-new** English
prose only, compose the `baoyu-translate` skill, which honors
`.baoyu-skills/baoyu-translate/EXTEND.md` (zh-CN, technical, glossary keeping
Provider / Agent / crate / harness / 会话). Do **not** free-regenerate a ZH file
— that desyncs the pinned tokens.

**Done when:** each ZH file mirrors its EN structurally and every pinned ZH
token is still present.

### Phase G — Verify

Run the guard suites (command in
[`references/doc-guards.md`](references/doc-guards.md)), then
`grep -nE "Phase [0-9]|第 ?[0-9]+ ?阶段"` across the edited docs to catch
phase-jargon the guards do not enforce — they check token *presence*, not
phase-label *absence*. If any edited doc is `include_str!`'d into rustdoc, also
run `cargo doc --workspace --no-deps` and `cargo test -p <crate> --doc`.

**Done when:** every guard suite reports `0 failed`, the grep is clean (or every
remaining phase ref is intentional and noted), and — if `docs/opi-spec.md` was
touched — the phase4 specification-hash ledger is re-synced.

## What it does NOT do

- No code, `Cargo.toml`, or version changes — `opi-release` owns the version
  bump.
- No commits, pushes, PRs, or releases.
- No authoring of `docs/opi-spec.md` normative content (Phase 1, or an
  `opi-implement` doc task, owns that). It may edit `opi-spec.md` only as a
  doc-sync, and then it **must** re-sync the live ledger spec-hash
  (`crates/opi-coding-agent/tests/spec_ledger.rs` pins
  `.opi-impl-state.json`) — any byte change there breaks that SHA256 guard.
  Phase-exit snapshots under `docs/snapshots/phaseN/` are historical; do NOT
  re-sync them.
- No weakening, disabling, or rewriting of guard-test files to "make docs
  cleaner". If a guard blocks a desired change, the guard wins; escalate.
- No free-regeneration of Chinese docs.

## Artifacts

- **Reads:** the affected `README*.md` / `docs/*.md`, `CHANGELOG.md`,
  `CLAUDE.md` / `AGENTS.md`, crate `src/`, the guard-test files under
  `crates/*/tests/`, and `.baoyu-skills/baoyu-translate/EXTEND.md`.
- **Writes:** the affected docs (EN + ZH). On an `opi-spec.md` edit, also the
  repo-root `.opi-impl-state.json` spec-hash (CRLF-normalized), pinned by
  `crates/opi-coding-agent/tests/spec_ledger.rs`. Phase-exit snapshots under
  `docs/snapshots/phaseN/` are historical and are NOT re-synced.

## In the workflow

Phase 6 — after Phases 3–5 (implement / audit / remediate) pass and before
`opi-release` (Phase 7). `opi-implement` Phase D does only per-task inline doc
verification; `opi-document` owns the dedicated, guard-verified doc refresh and
the EN/ZH sync. `opi-release` Phase 5 stages version files but defines no
doc-sync step of its own — run `opi-document` first.
