# ADR-0002: Task-scoped context instead of global spec admission

Status: accepted — requested by the maintainer in this documentation migration.

## Decision

The root guidance routes to focused engineering rules, a domain glossary,
relevant ADRs and one user-agreed task/iteration. Source, tests, schemas and
manifests establish current facts. The user's goal and observable acceptance
establish desired change; material conflicts are resolved with the user and
recorded at their owner. New work does not require a global spec revision,
Phase registration or `opi-implement plan`.

This replaces the authority hierarchy in legacy `AUTH-003`/`AUTH-004` and
Phase derivation rules in `docs/opi-spec*`. Remove the retired specifications
and `docs/CONTEXT.md` from the checkout; Git retains their committed versions.
Keep current guidance focused on current practice rather than retirement notices.
Existing ledgers, snapshots and their hashes are not rewritten by this migration.
Existing runtime safety and verification are not relaxed by retiring a roadmap.

## Why

Rapid implementation and upstream changes make a global prose inventory costly
to synchronize. Splitting the same inventory into several mandatory documents
would reproduce the problem. We instead separate meaning, decisions, facts and
temporary scope, with conditional loading and same-change maintenance.

A feature-specific spec remains useful for multi-session work. Matt's wayfinder
is an optional decision map, not a required per-feature ceremony. Its project
adapter stores durable local artifacts under `docs/iterations/`; `.scratch/`
is disposable. Engineering, glossary and context files have one English copy;
existing bilingual documents remain synchronized.

## Consequences

- Current docs must be updated or pruned when their owning behavior/decision
  changes; terminal iteration briefs stop acting as current scope.
- Legacy Phase tooling remains available only for explicitly selected legacy
  work under its [boundary](../../.agents/skills/_shared/references/legacy-phase-workflows.md).
  Missing old inputs remain visible blockers for that workflow, not blockers
  for new work. A later tool migration is a separate decision.
- Checks validate current documentation structure/links and product contracts;
  they do not require deleted documents or retirement notices to remain present.
- This does not implement a product feature, change runtime contracts, migrate
  existing tasks, authorize Git publication or establish automatic skill-path
  compatibility. Validate the Matt adapter on a real requested iteration.

[中文](0002-task-scoped-context.zh.md)
