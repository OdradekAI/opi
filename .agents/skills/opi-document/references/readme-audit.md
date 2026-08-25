# Full README audit

Use this reference only for `scope=full`. A full audit is independent of
outgoing-change discovery: it verifies every maintained current-product README
against current implementation evidence even when the file and its owning code
do not appear in a diff.

## Build the inventory

Use `git ls-files` to enumerate tracked files whose basename is `README.md` or
`README.zh.md`:

```text
git ls-files | rg '(^|/)README(?:\.zh)?\.md$'
```

Exclude frozen snapshots, generated artifacts, and inward or outward evidence
whose original wording is the historical record. Never exclude a README merely
because it is unchanged. Report every excluded path and the exclusion reason.

Group English/Chinese counterparts, but keep one coverage row per path so every
included README is visibly reviewed. Root and Cargo-package README pairs must
remain present. Other current README files, including example and workflow
guides, are audited as tracked; a missing Chinese counterpart is drift only
when a maintained counterpart contract requires one.

Read every included README completely before judging it. Search results may
route attention, but they are not a substitute for reading the complete
propositions in the file.

## Evidence routing

Map each checkable claim to the narrowest authoritative owner:

| README surface | Primary evidence |
|---|---|
| Root product README | Cargo metadata, generated `opi --help`, CLI/config behavior tests, normative spec |
| Crate README | Crate manifest, public source/rustdoc, owning behavior or conformance tests |
| Example README | Example manifest/source plus the public API or CLI seam it demonstrates |
| Skill/workflow README | Owning `SKILL.md`, sidecars, schemas, scripts, and workflow tests |
| Other current README | The source, schema, generator, or behavior test that owns each claim |

Audit versions, workspace topology, public APIs, commands and flags, defaults,
ordering, failure and safety guarantees, compatibility, platform support, and
runnable examples wherever they appear. Changelog entries, test names, prior
documentation audits, plans, and snapshots may locate a claim but do not prove
current behavior.

For each complete proposition, record its owning evidence and classify it as
`keep`, `drift`, `noise`, `gap`, or `defer`. Apply the prose contract's edit
actions to confirmed changes. A claim without sufficient current evidence is
`defer`, not `keep`.

Review English and Chinese files semantically. Counterpart existence or similar
heading structure does not prove that conditions, negative guarantees,
exceptions, failure behavior, or consequences remain synchronized.

## Coverage matrix

Include this matrix in the handoff:

| README | Counterpart | Sources checked | Result | Evidence or edit |
|---|---|---|---|---|
| `<path>` | `<path>` or `none` | `<owning paths/checks>` | `keep`, `updated`, or `defer` | `<source citation, change, or limitation>` |

Every included README has exactly one row. List excluded README files separately
with their exclusion reason. `updated` rows identify whether the edit was
`add`, `trim`, `restore`, or `restructure` and cite the source of truth.

The full audit is complete only when:

- the tracked README inventory was recomputed after edits;
- every included file was read in full and has one coverage row;
- every checkable claim was compared with owning evidence;
- every confirmed drift or actionable gap was fixed in the authoritative
  English/Chinese unit, or remains an explicit `defer` limitation;
- all surface-specific verification and the documentation checks passed.

No documentation changes is a valid result only when every included row is
`keep`. Mechanical checks alone never establish this result.
