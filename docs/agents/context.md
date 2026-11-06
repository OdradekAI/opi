# Task-scoped context

## Start or resume

1. Establish the requested outcome, affected surface and observable acceptance.
   A clear user request is sufficient for a small change; do not demand a spec,
   ADR, ticket or Phase registration for routine work.
2. Inspect current implementation/callers/tests. Read only relevant glossary
   entries, engineering guidance and decisions. Distinguish observed behavior
   from desired behavior; neither a passing test nor an old design settles a
   disputed requirement.
3. For multi-session work, material ambiguity, or a public-contract change,
   create/update one [iteration brief](../iterations/README.md). Record the
   agreed scope, open decisions and source identity, not a transcript. Use an
   existing issue as the brief when the user supplied one; link it rather than
   copying its state. Do not infer the active task from a newest-file scan.
4. On resume, compare the brief's baseline and affected sources with current
   code and the user's latest decisions. Recheck only invalidated assumptions.
   Mark stale sections before relying on them; a date/hash is provenance, not
   proof of correctness. Done when acceptance is clear and material conflicts
   have a decision or an explicit blocked owner/question.

## Resolve a conflict where it occurs

Identify both claims and their source paths, show a concrete scenario where
they differ, recommend the smallest coherent resolution, and ask one actionable
question when scope, compatibility, safety, authority or an expensive trade-off
would change. Do not answer for the user or silently prefer the newest prose.
Continue independent work while that decision is blocked.

- Verifiable factual drift: fix the current documentation from source and report
  it; no repeated approval is needed for an already authorized correction.
- Changed product meaning or an accepted ADR: obtain the user's decision, then
  update the brief/contract and mark any superseded decision explicitly.

## Build and close

Implement the smallest coherent slice; improve module depth/locality within its
scope. Verify at the real consumer interface, including negative behavior at
changed boundaries. Align names, tests, schemas, examples and existing language
mirrors in the same change.

At each meaningful decision or handoff:

- Put resolved domain meanings in the [glossary](../GLOSSARY.md).
- Record an ADR only for a costly-to-reverse, surprising, genuine trade-off.
- Put shipped behavior in the owning product docs; keep code facts out of the
  glossary and temporary plans out of engineering standards.
- Update the brief's remaining decisions and checks. When done/superseded,
  remove it from any active map, link its outcome/successor and stop loading it
  as current requirements. Do not erase historical evidence or unrelated work.

A handoff names outcome, remaining blocker, next action and exact checks. Keep
one selected task brief or issue as the owner of progress, rather than copying
its state into another tracking system.

## Optional Matt flows

Read [tracker](issue-tracker.md) and [domain](domain.md) configuration first.
`ask-matt` is a router, not another approver. Use the selected skill's actual
instructions; available commands vary by installation.

- Bounded ambiguity: conversation, optionally requested `grill-with-docs`.
- A large unresolved decision space across sessions: requested `wayfinder`;
  its map resolves decisions, not implementation tasks.
- A settled multi-session delivery: a scoped brief/spec and, when needed,
  self-contained tickets. A feature spec is useful; a mandatory global spec is
  not. Each ticket links acceptance, owned scope, relevant decisions and blockers.
- A small settled change: implement and verify directly.

Skill output is not human approval. Publication, credentials, paid calls and
irreversible actions retain their own explicit authorization.
