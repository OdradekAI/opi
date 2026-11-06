# Documentation verification reference

Choose the cheapest evidence that proves the current claim.

| Claim | Authoritative evidence | Verification |
|---|---|---|
| Documented version/schema values, paired docs for every `crates/*/Cargo.toml` package, local link targets, root-guidance content consistency | Cargo/source plus maintained docs | `python scripts/opi-doc-check.py` |
| Project-local skill names, explicit invocation metadata, Codex sidecars, and EN/ZH skill-index membership | `.agents/skills/opi-*/SKILL.md`, `agents/openai.yaml`, and the two skill indexes (also read through the `.claude` alias) | `python scripts/opi-doc-check.py` |
| Public Rust API and examples | Rust items and rustdoc | crate-scoped `cargo test --doc` / `cargo doc` |
| Runtime, CLI, provider, session, tool, or safety behavior | Owning behavior/integration test | named test binary or test filter |
| Architecture boundary | Cargo metadata or topic-based contract test | focused architecture check |
| Historical phase decision | frozen design/plan/snapshot | no current-product guard |
| Semantic prose quality | Owning source plus the complete-proposition review in `references/prose-contract.md` | scoped semantic judgment; mechanical checks only verify applicable structure and routing |

## Mechanical check boundary

The checker validates required files and inline local-link targets in root/crate
READMEs, current context documents and skill Markdown, including references.
It does not validate external URLs, section anchors or every Markdown link form;
links into the ignored `.repo/` evidence cache remain exempt.

Each paired README must contain the current workspace version as an inline code
value; surrounding wording is free. Root README schema assignments must match
the owning Rust constants, including every repeated assignment. These checks
detect missing/drifting values, not contradictory prose or translation drift.
Root-guidance comparison ignores whitespace and warns on a materialized alias.

Workflow prose and historical migration vocabulary are reviewed against their
owning contracts, without required/forbidden sentence lists. Workflow helper
behavior belongs to the owning skill's tests. Product source tripwires belong
to [crate boundary tests](../../../../crates/opi-coding-agent/tests/crate_boundaries.rs);
they remain limited source checks, not behavioral proof. CI workflow placement
belongs to [CI contract tests](../../../../crates/opi-coding-agent/tests/ci_contract.rs).

For checker changes, run `python scripts/test_opi_doc_check.py` (including its
CLI failure-path test), the checker itself and `git diff --check`. Routine CI
runs both the checker and its regression suite.

## Rules

- Never encode exact narrative sentences, roadmap placeholders, historical
  non-goals, released changelog text, or test function names in a Rust test.
- A current safety or compatibility statement must derive from current source
  or a behavior test. Merely finding a phrase in a README is not evidence.
- `scripts/opi-doc-check.py` is intentionally narrow and fast. Add a rule only
  when the claim is stable, source-derived, user-significant, and cheaper than
  the failure it prevents.
- Automated checks may confirm that the prose workflow remains wired, but they
  do not prove meaning, completeness, translation quality, or editorial
  judgment.
- When a rule becomes obsolete, delete or replace it in the same change as the
  product/doc transition. Do not accumulate phase-specific compatibility
  clauses.
- `docs/snapshots/phaseN/` and completed implementation plans are historical;
  references to deleted historical tests may remain there.

## Test-impact decision

Every feature, refactor, or removal records one of:

- `add`: new observable behavior needs new coverage;
- `update`: an existing current contract changed;
- `delete`: the old behavior/claim no longer exists;
- `retain`: existing coverage already proves the unchanged contract;
- `none`: documentation/skill/metadata-only change with no runtime contract.

`none` still runs the fast documentation check when maintained docs or skills
change. It does not justify a workspace Cargo test.
