# Realign report template

Write `docs/research/YYYY-MM-DD-opi-<opi-id>-vs-pi-<pi-id>-<scope>.md`.
Use short content identities tied to the recorded manifests, not a version name
alone. Sanitize the scope token; append a run suffix on collision instead of
overwriting an existing report. Keep supporting evidence beside the report in
a correspondingly named directory. These artifacts are not a progress ledger.

Write in the user's requested language. State whether a bilingual counterpart
is supplied or why this invocation's generated evidence is single-language;
maintained English/Chinese documentation is not edited by the audit.

## Header and reading guide

- Measurement date, resolved pi/Opi roots and declared package versions.
- Content identities and manifest locations, inclusion/exclusion rules,
  working-tree state, verified Git metadata if available, generated-data and
  executable identities separately. Unknown upstream provenance is explicit.
- Requested scope, discovered surfaces, dimension/contract counts and exclusions.
- Method and executed checks; source-only, self-review and unavailable-material
  limitations. Historical reports and Opi planning documents are not inputs.
- Explain A = current facts, H = target design, B = conditional engineering advice.
  The target is full reproduction in scope, not Opi-spec compliance. Neither a
  complete survey nor advice authorizes development or proves conformance.

Use the framework's three completion fields: coverage, review coverage and
reproduction conclusion. Also summarize the independent acceptance gates;
proposed or unexecuted checks are not passing evidence.

## Coverage inventory

`Contract ID | Dimension(s) | pi surface/entry | Opi mapping | Exposure/consumer |
Evidence basis | Coverage or exclusion reason`

Expand public methods/operations and meaningful subcriteria, not just dimension
headings. Include Opi-only observations and explicit empty/absent search scopes.
State source-derived versus actual installed-package export coverage. Missing
materials, unidentified surfaces and interrupted batches stay visible.

## Layer A: current contracts and differences

Group by dimension, cross-referencing shared contract IDs rather than counting
them twice. Provide a concise overview and the detailed records needed to make
claims independently understandable:

`ID | pi evidence | Opi evidence | Comparison | Observable difference |
Evidence basis | Review outcome`

For a nontrivial item, add an invocation trace and a parameter/behavior table:

```text
caller -> normalization -> auth/policy -> dispatch -> operation -> result/cleanup
```

`Aspect | pi | Opi | Consequence`

Cover applicable defaults/precedence, inputs, callback/event ordering, state,
errors, concurrency, cancellation/drop and persistence/cleanup. Show how a caller
can distinguish the implementations even when the feature exists on both sides.
Reference source anchors, bounded absence searches and executable results
separately. An unexecuted proposed test belongs in B, not as A's result.

## Layer H: snapshot-local target design

Record searched design paths, including when no qualifying material is found.
For each item use:

`ID | Design claim/anchor | Stated authority/status | Current implementation |
Observed maturity | Public/product exposure | Opi evidence | Review outcome`

Use the framework's authority, maturity and review rules. Keep implemented
experimental code distinct from design-only claims, and library implementation
distinct from actual CLI adoption. Describe explicit non-goals and open questions
without manufacturing parity conclusions. Preserve source/design conflicts.

## Layer B: reproduction and engineering advice

Included by default unless the user asked for facts alone. Use the conditional
cards in `acceptance.md`; every card references A/H finding IDs. Include:

1. Capability/workflow/parameter changes needed for full reproduction.
2. Architecture assessment of actual responsibility, production/build/test
   dependencies, interfaces, state and resources, including sound structure to
   retain and evidence-backed alternatives to improve it.
3. Acceptance records, negative cases, proof layers, gate conditions, test impact,
   platform/resource budgets or the measurement needed to define them.
4. Technical prerequisites and suggested verification order, with uncertainty,
   migration/rollback costs and safety-versus-equivalence conflicts explicit.

Describe hypothetical changes as proposals, not current facts. Do not require
Opi's current specification to approve a recommendation, waive a target gap
because it is absent from the roadmap, or turn advice into an approved task.
Differences retained for safety or Rust usability remain visible in parity
results. Only user scope, not project intent, narrows the reproduction claim.

## Method, limitations, and refutations

Record coverage denominators; comparison, maturity and review outcome tallies;
independent/self/unreviewed IDs; refuted claims; unavailable evidence and
unresolved conflicts. Give actual commands, features, exits and execution counts
for checks performed, fixture/result identities, and supporting artifact links.

Review the whole report for evidence/advice leaks in its language, not only an
English keyword list. Check every recommendation's evidence links, claimed
completion against the inventory, content identities at handoff, and report-path
collisions. No `pi ahead`/`Opi ahead` quality ranking follows from feature counts.

## Chat summary

Lead with concrete capability, invocation-flow, parameter and behavior differences,
then summarize architecture findings, conditional ways to close gaps, and their
acceptance criteria. State the actual snapshot identity and design maturity,
coverage/execution/independence limitations, unresolved questions and report link.
The user receives a usable engineering assessment without needing historical
context, a roadmap review, or immediate implementation admission.
