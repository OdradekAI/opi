# Remove historical alignment labels from public opi-ai manuals

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Split the Provider, Catalog and Auth reference](33-readme-provider-auth-topic.md)

## Scope

The package-facing README and topic references still exposed historical task,
phase, acceptance and snapshot identifiers such as `20.35`. Those identifiers
belonged to the internal alignment ledger, not to the opi-ai onboarding or API
contract, and made the manuals read like an implementation diary.

## Implementation

The six public opi-ai manuals now describe current capabilities, observable
behavior, public entry points and package-local references without internal
alignment labels. Historical traceability remains in
`docs/iterations/opi-ai-1-5-alignment/`, its issue/report/evidence files, and
Git history. This keeps the public manual aligned with the Pi style of a
capability-oriented developer guide while preserving an auditable internal
record.

## Acceptance

`test_public_opi_ai_manuals_omit_internal_alignment_labels` scans both
localized README files, the cost/pricing and Provider/Auth topic references,
and the three shipped onboarding examples for the retired label families and
phase/snapshot markers. `opi-doc-check.py` validates the resulting
documentation links, while the packaged-consumer proof checks that both
localized manuals still embed the exact examples.

## Limits

This is a documentation and contract-test cleanup. It does not change runtime
behavior, public Rust symbols, feature selection, pricing, provider/auth
semantics or the internal alignment records.
