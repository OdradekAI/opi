# Documentation verification reference

Opi uses three kinds of documentation evidence. Choose the cheapest kind that
proves the current claim.

| Claim | Authoritative evidence | Verification |
|---|---|---|
| Current version, schema constant, paired docs for every `crates/*/Cargo.toml` package, local links, root-guidance lockstep | Cargo/source plus maintained docs | `python scripts/opi-doc-check.py` |
| Project-local skill names, explicit invocation metadata, Codex sidecars, and EN/ZH skill-index membership | `.claude/skills/opi-*/SKILL.md` or `skill.md`, `agents/openai.yaml`, and the two skill indexes | `python scripts/opi-doc-check.py` |
| Public Rust API and examples | Rust items and rustdoc | crate-scoped `cargo test --doc` / `cargo doc` |
| Runtime, CLI, provider, session, tool, or safety behavior | Owning behavior/integration test | named test binary or test filter |
| Architecture boundary | Cargo metadata or topic-based contract test | focused architecture check |
| Historical phase decision | frozen design/plan/snapshot | no current-product guard |
| Semantic prose quality | Owning source plus the complete-proposition review in `references/prose-contract.md` | scoped semantic judgment; mechanical checks only verify applicable structure and routing |

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
