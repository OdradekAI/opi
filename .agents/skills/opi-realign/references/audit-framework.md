# Audit framework: snapshot, contracts, and evidence

## Snapshot identity

The supplied path, not a release name or a separate document's baseline, selects
pi. Record each root, declared package versions, and a reproducible content
identity for the audited source/material set. Retain a relative-path/hash
manifest and its exclusions with supporting evidence; distinguish source,
lockfile, generated model data, and executed artifacts. Missing generated data
can block execution or catalogue claims without blocking source inspection.

For a Git tree, verify which repository owns the root before recording its HEAD
or remote. A snapshot nested inside Opi may inherit Opi's `git rev-parse`
result: that is not pi provenance. Record relevant staged, unstaged, untracked,
and ignored build inputs; a commit alone does not identify their content. For
an export without Git, use the content manifest and mark upstream SHA/remote
unverified. Do not demand a clone, fetch, or checkout to repair that metadata.
An explicitly named revision that disagrees with the supplied tree requires
clarification, not silent substitution.

Use a stable read-only copy when concurrent changes threaten consistency;
otherwise recheck input hashes before handoff and remeasure changed evidence.
Exclude report output, dependency/build caches, and secrets from the source
manifest, stating the boundaries. If a required input cannot be identified,
mark the affected conclusions unresolved rather than inventing a pin.

## Report layers

- **A — current contracts.** Snapshot-bound implementation and observable
  differences, including missing and Opi-only capabilities. No adoption advice.
- **H — design horizon.** Active pi design material in this snapshot, its own
  stated authority/status, and implementation maturity. Never an Opi requirement.
- **B — conditional recommendations.** How to reproduce the target capability,
  improve engineering structure, and verify the result. Included by default;
  omit only if the user explicitly requests facts alone. Recommendations are
  engineering judgments with evidence and trade-offs, not objective facts or
  product priorities. Technical prerequisites may order the suggested work.

Use the source rules in `SKILL.md` for all three layers. In particular, Opi's
specification, plans, and historical conclusions neither waive a difference nor
veto an architectural alternative. Full reproduction is the assessment lens;
the user retains the decision to change product scope or adopt the advice.

## Inventory and claim records

Inventory first, then measure. Expand manifest exports, wildcards and recursive
re-exports against available source/build artifacts; inspect public methods,
CLI operations, fixtures and actual consumers. Label source-derived export
inventories as such when the installed artifact cannot be inspected. Internal
exports do not automatically require a public Rust equivalent. Give discovered
contracts stable descriptive IDs within the report; a dimension heading is not
a contract or a proof of coverage.

For each contract, record:

- pi entry and Opi mapping; public/internal/tooling status, experimental status,
  activation conditions, and demonstrated product consumers;
- caller workflow, parameters/defaults/precedence/valid inputs, capabilities,
  outputs, event/callback order, errors and their stage;
- state changes, external effects, ownership, concurrency, cancellation/drop,
  persistence and cleanup; relevant platform and host conditions;
- both-side source anchors, original tests/fixtures, evidence basis, review
  outcome, remaining questions, and linked recommendation/acceptance IDs.

Classify the **current comparison**, independently of review strength:

| Result | Meaning |
|---|---|
| `equivalent` | All assessed applicable observations agree; state the evidence limits. |
| `partial` | Only a subset of the target capability is present. |
| `missing` | The target capability is present; bounded source search found no Opi implementation. |
| `different` | Both implement the capability, but at least one observable contract differs. |
| `opi-only` | Opi provides a capability not found in the snapshot search scope. |
| `unresolved` | Available evidence cannot establish the comparison. |

Do not use `Out of Scope` or presumed intent to hide a gap. User-excluded scope
is listed outside the denominator; snapshot-not-applicable needs source/search
proof. A deliberate or beneficial behavior change remains `different` for strict
reproduction. Different module layout alone is not a behavioral difference.

## Design horizon

For each consequential in-scope design item, record the artifact's own wording
about authority (normative, draft, explanatory, informative, open question,
non-goal, or unstated) separately from observed maturity:

- `implemented`: matching implementation and focused test evidence exist;
- `partial`: a meaningful subset is implemented;
- `scaffold`: shape exists, but primary operations are placeholders;
- `designed-only`: no implementation found in the stated search scope;
- `not-assessed`: implementation comparison is inapplicable, such as a non-goal;
- `unresolved`: implementation comparison was blocked or evidence conflicts.

These labels do not assert tests were run or product adoption occurred. Cite the
design and implementation/search evidence; a title or directory does not confer
authority. Superseded proposals, external roadmaps and generic TODOs are not
active design merely because they are mentioned. Record the searched design
paths even when a scoped audit finds no qualifying document. Implemented
experimental behavior belongs in A too, with its exposure and maturity clear.

## Verification

Use separate measurement and adversarial-review passes for every scope. An
independent verifier reads the pinned primary evidence afresh, with the scope
and claims but without inherited roadmap or historical verdicts. A second pass
by the same agent is useful but is **self-review**, not independence; unreviewed
items remain visible. Host capacity limits concurrency, not required coverage.

Assign each claim `confirmed`, `refined`, `refuted`, `added`, or `unresolved`.
Correct refined claims; verify added claims by the same standard. Keep refuted
claims in method notes rather than the retained findings. Confirming a source
fact does not establish runtime conformance.

Record evidence bases separately: source inspection, test-code inspection,
executed deterministic probe, paired differential run, assembled-product run,
external-package consumer, and authorized live run. These are distinct proofs,
not interchangeable ranks. Existing results require matching source, material,
configuration, fixture, command and result identities before reuse.

For absence, record searched paths, terms, alternate names and inspected
registration/dispatch/consumer routes. A failed search or missing dependency is
not absence. For parity, actively seek a differing parameter, failure phase,
state transition or consumer path. For source/doc/test conflicts, preserve both
anchors, use controlled execution where possible, and leave unresolved cases
open; never silently select the most convenient expected result.

## Completion

Report three independent conclusions:

- **Coverage complete/incomplete:** all discovered in-scope surfaces mapped and
  investigated, with evidence-backed exclusions; missing/unreadable surfaces
  make coverage incomplete. Naming all dimensions is insufficient.
- **Review coverage:** independent, self-reviewed, mixed, or incomplete, with
  counts and uncovered IDs.
- **Reproduction established/not-established:** full reproduction requires all
  applicable contracts to pass their executable acceptance with no unresolved
  differences. A complete source survey can still leave this not-established.

Unresolved evidence is a valid audit output, never a passing conformance result.
Performance, safety and structural quality are reported separately from parity;
a weighted score cannot cancel a failure in another gate.
