# Shared decision and test stewardship

This reference has `decision-and-test-closure-only` authority. It helps an
owning workflow concentrate one semantic decision and its tests at one
Interface. It does not admit product scope, own implementation progress,
authorize a public seam, or authorize test deletion by itself.

Read the complete reference when:

- `opi-implement` plan admission detects a shared-decision trigger;
- `opi-implement` is about to write the first red test for a runtime task;
- task or phase verification evaluates decision locality or test disposition;
  or
- `opi-slim-tests` classifies candidates or consumes a slim handoff.

## Shared-decision trigger

Record `field=shared_decision` only when at least one trigger is present:

- `intrinsic-state`: the decision is intrinsic Agent state-machine semantics;
- `multiple-consumers`: at least two task participants have real production
  call sites that consume the same rule;
- `expand-contract`: a reviewed sequence temporarily creates parallel paths;
  or
- `recurrent-finding`: the same semantic decision failed verification again.

An ordinary one-consumer helper keeps its concrete implementation and records
no shared-decision note. Tests, docs, examples, and fixtures do not count as
production consumers.

## Plan note contract

Each participating task records one existing-shape `{ field, reason, source }`
note per decision:

```text
field=shared_decision
reason=decision_id=<stable semantic id>;
       role=owner|consumer;
       owner_task=<task id>;
       module=<owning Module>;
       interface=<owning Interface>;
       representation=<enum|newtype|state machine|fallible value>;
       consumer_tasks=<comma-separated task ids|none>;
       criterion_ids=<comma-separated sourced scenario ids>;
       legacy_paths=<comma-separated paths|none>;
       closure_test=<exact behavioral-test path or verification command>;
       trigger=intrinsic-state|multiple-consumers|expand-contract|recurrent-finding
source=<registered source heading>
```

`decision_id` names semantic ownership, not a task, test, or finding instance.
All notes for one ID agree on every clause except `role`. Exactly one task uses
`role=owner`; `consumer_tasks` excludes that owner and exactly matches tasks
using `role=consumer`. Every consumer depends transitively on the owner.

The owner exposes one typed Interface and owns `closure_test`. The declared
criteria appear in participating tasks' acceptance scenarios. An
`expand-contract` decision names every temporary or legacy path that must close.
Every participating task is evaluator-required because local mechanical tests
cannot prove assembled decision locality.

Plan admission is complete when the deterministic graph validator accepts the
note structure and an adversarial reviewer confirms the cited owner, consumers,
Interface, representation, and repository evidence are true.

## Test candidate classification

Read candidate bodies in full. Give every candidate exactly one primary
classification:

- `current-contract`: proves observable shipped behavior or a live public
  protocol, Interface, safety, security, or persistence rule;
- `duplicate`: another test reaches the same Interface with materially
  equivalent fixtures and assertions;
- `superseded`: pins removed behavior, an old Interface, or implementation
  detail below the current Interface;
- `historical-evidence`: asserts phase status, old roadmap prose, or delivery
  history that belongs in a frozen artifact;
- `platform-only`: proves a real OS or toolchain contract; or
- `helper-binary`: supplies a subprocess fixture rather than assertions.

Names, file proximity, and shared setup are not evidence of equivalence.

## Test impact

For each behavior affected by the current product change, choose one action:

- `retain`: an existing test expresses the current contract and can provide the
  required red proof;
- `update`: the observable contract changed, so update its existing test;
- `add`: no current test expresses a distinct new observable case;
- `delete`: the behavior or Interface is superseded and equal-or-stronger
  replacement proof already passes; or
- `none`: the task changes no runtime contract.

## Test replacement proof

Apply replace-don't-layer at the Interface test surface. Prefer making an
existing Interface test red
before adding another test. Add a case only when its observable input, outcome,
failure classification, ordering, or authority fact is materially distinct.

Establish the replacement Interface test before removing a predecessor. Keep
current safety, security, protocol, persistence, platform, and reviewed
snapshot guards unless the replacement proves the same rule at least as
strongly. A deletion that merely makes a failing implementation green has no
replacement proof and is refused.

Test design is complete when every affected behavior has one action, every
added or updated test crosses the owning Interface, every deletion names its
passing replacement, and no equivalent old/new contract tests remain in task
scope.

## Runtime output and slim handoff

Record every test touched or intentionally retained by an `opi-implement` task
in `session_notes[].gate_results.test_disposition`:

```json
{
  "subject": "<decision id|scenario id|task id>",
  "action": "add|update|delete|retain|none",
  "test": "<path or test identity|none>",
  "interface": "<owning Interface|not-applicable>",
  "replacement": "<passing replacement identity|null>",
  "slim_candidate": "duplicate|superseded|null"
}
```

`opi-implement` replaces or removes a task-local duplicate or superseded test
only after replacement proof passes. A candidate outside task-owned paths stays
unchanged and records `slim_candidate=duplicate|superseded`.

`opi-slim-tests` treats that handoff as an inventory hint. It rereads the full
candidate and retained proof, classifies both independently, and reports a
false or stale hint instead of deleting from trust. Broad integration-binary
consolidation remains solely `opi-slim-tests` work.
