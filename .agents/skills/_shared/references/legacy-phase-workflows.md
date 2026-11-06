# Legacy Phase workflow boundary

The project now uses [task-scoped context](../../../../docs/agents/context.md).
`opi-implement`, `opi-audit` and `opi-remediate` retain their existing
Phase formats and helper protocols for explicitly selected legacy work. They
are not admission, review or repair prerequisites for new features.

## Before using legacy procedures

- Confirm that the user wants the existing Phase/ledger or indexed finding set,
  rather than a new ordinary iteration. Explain the historical source contract
  it will apply. Read-only status does not need a new mutation approval.
- Check that all registered source files exist at the selected baseline.
  `docs/CONTEXT.md` and `docs/opi-spec*` have been removed from the checkout;
  committed versions remain in Git, as do earlier Phase research inputs.
  For read-only reconstruction, explicitly select the matching commit and use
  an isolated export. Do not restore retired sources into the current tree or
  substitute current guidance for sealed requirements. Missing inputs block
  that operation; do not reconstruct authority from memory or invent registrations.
- References to “normative”, “latest committed spec” and admission in the
  procedures below mean that explicitly selected legacy contract only. They
  do not restore the retired spec as current project policy. If current policy
  and the requested legacy operation conflict materially, ask which contract
  the user intends to revise before proceeding.
- Source removal does not authorize resyncing ledger hashes, rewriting
  snapshots, automatic drift reconciliation or task completion. A user choosing
  mutating work must first settle source applicability and the source-drift plan
  through `opi-implement`; snapshots keep their original pins.
- Preserve the selected workflow's source binding, review, verification and
  Git/publication gates. Only `opi-implement` writes its ledger. Moving to a
  new task brief is a user decision, not evidence that an old Phase passed.

Current feature work instead uses the user-agreed request/brief, relevant ADRs,
current source and focused acceptance checks. Do not copy new work into the old
ledger or force it through a legacy registration list.
