# Iteration briefs

Create a brief only for work that needs durable alignment: multi-session scope,
material ambiguity, or a public-contract change. A routine edit needs only a
clear request and a verification handoff. There is no required global task list.

Use `docs/iterations/<slug>/brief.md`, or `spec.md` when a requested Matt
flow writes one. They are alternatives, not two copies. A supplied tracker
issue can own the brief instead. Maps/tickets follow the
[tracker contract](../agents/issue-tracker.md).

## Minimal brief

Use the headings needed by the task; expand only where ambiguity requires it.

```markdown
# <Outcome>
Status: draft | active | blocked | done | superseded
Owner: <decision owner / user>
Baseline: <source commit and relevant uncommitted changes>

## Goal and acceptance
<User-visible outcome; executable checks; explicit non-goals.>

## Context and decisions
<Relevant source paths, terms and ADRs. Distinguish current facts from proposals.
For upstream alignment: actual repository/path, revision and compared surface.>

## Open questions
<Conflicting claims, recommendation, decision owner, and what is blocked.>

## Work and verification
<Small slices: step -> verify: check. Tickets if dependency/ownership needs them.>

## Handoff
<Exact checks/results, remaining gaps, next action or outcome/successor links.>
```

`draft` is not approval. `active` records the user's agreed scope, not
permission for commits, publication or paid calls. `blocked` names an unresolved
decision; independent work may continue. `done` requires acceptance evidence or
an explicitly accepted limitation. `superseded` links the replacement.

On resume, reconcile changed assumptions with source. On completion, update
owning current docs and mark this brief historical through its terminal status.
Never copy its task list into the glossary, engineering guide or old Phase ledger.

## Archived opi-ai 1.5 alignment

Read the iteration spec at its original revision.

```text
git show 8d3597487f575eb4e8931b293f670780066cb93b:docs/iterations/opi-ai-1-5-alignment/spec.md
```

List the corresponding issues, evidence and tools.

```text
git ls-tree -r 8d3597487f575eb4e8931b293f670780066cb93b -- docs/iterations/opi-ai-1-5-alignment
```

Run new checks with the [SDK conformance tools](../../scripts/pi-sdk-conformance/README.md).
