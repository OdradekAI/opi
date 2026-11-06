# Issue tracker: project-local Markdown

For requested Matt planning flows, use `docs/iterations/<slug>/` as the local
tracker. This implements the user's docs-local preference; it does not change
where public bug reports live or authorize remote issue/branch/PR creation.

## Path adapter

- One effort per directory; its single brief is `brief.md` or `spec.md`,
  never both with duplicate scope. See [brief lifecycle](../iterations/README.md).
- Implementation tickets: `issues/NN-<slug>.md`, one per ticket. Include
  `Status: open|claimed|blocked|done|superseded`, owner, acceptance and
  `Blocked by:` links. Claims are saved before work; completion needs checks.
- Comments/decisions belong in the owning ticket, not copied into a global log.
- Fetch a ticket by its supplied path. Local “publish” means write the file;
  it never means commit, push or create a remote issue.

Matt's local templates and `to-tickets` include literal
`.scratch/<feature>/` paths. For this project, translate those *planning
artifact* paths to `docs/iterations/<slug>/` before writing. If an installed
script cannot use this location, report the mismatch rather than creating a
second tracker or editing global skills. `.scratch/` stays ignored, disposable
workspace for probes/drafts; durable decisions must not exist only there.

## Wayfinding operations

- Map: `docs/iterations/<slug>/map.md`, with destination, notes, decisions-so-far,
  not-yet-specified and out-of-scope sections. It is an index, not a second store.
- Decision tickets: `decisions/NN-<slug>.md`. Keep them separate from
  implementation `issues/` so planning cannot look like delivery.
- Metadata: `Type: research|prototype|grilling|task`,
  `Status: open|claimed|resolved|superseded`, owner and `Blocked by:` links.
- Frontier: open, unclaimed tickets whose blockers are all resolved; select
  lowest number unless the user chooses otherwise. Mark claimed and save before
  work; coordinate one writer rather than treating files as an atomic scheduler.
- Resolve: append `## Answer`, record the decision/evidence, mark resolved, and
  add one gist+link to the map. Human-in-the-loop decisions require the user.
- When the map clears, hand off to the brief/spec; do not auto-start delivery.

Read `domain.md` in this directory for glossary/ADR paths. No triage automation,
remote labels or alternative execution engine is configured by this document.
Requested flows retain their own approval gates and the repository's Git safety.
