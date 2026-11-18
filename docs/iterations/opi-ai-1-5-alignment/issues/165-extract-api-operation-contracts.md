# #165 Extract API operation contracts from Models

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)

## Goal

Make `api` the canonical owner of operation contracts before changing provider
profile state or the Models facade. This slice moves the chat operation seam
(`ChatApi`, neutral options, typed chat handles and native options) into
`api::contract`, removes the old `models::ChatApi` ownership, and keeps the
public root facade intentionally selective. Deferred contracts follow in the
next slice because their call-state and value types need the same boundary
decision.

## Evidence and scope

The current `models.rs` owns the `ChatApi` trait and its typed option handles,
while every protocol adapter implements that trait. `api.rs` owns only image
and classifier contracts and imports `models::ApiId`. The moved symbols have
real consumers in every built-in chat adapter, `opi-ai` integration tests,
`opi-agent`, `opi-coding-agent`, examples and the external consumer harness.

This slice does not change request execution, provider selection, wire
serialization, retry policy or authentication. `Models` remains the dispatch
facade. `SimpleOptions` remains a neutral value contract; its request mutation
helper is crate-private and is not part of the adapter interface.

## Canonical result

- `opi_ai::api::contract::{ChatApi, ChatApiHandle, NativeOptions,
  SimpleOptions, SimpleToolChoice}` owns the chat operation contract.
- `opi_ai::api::{ChatApi, ChatApiHandle, NativeOptions, SimpleOptions,
  SimpleToolChoice}` and the selected crate-root facade re-export the canonical
  contract intentionally; `opi_ai::models::{ChatApi, ...}` is removed.
- Internal adapters use `api::contract` and `types::ApiId` directly.
- The old implementation block is deleted from `models.rs`; its registration
  and execution owners consume the canonical contract through crate-private
  imports.

## Verification

- Chat adapter, custom registration, preparation and lifecycle tests pass with
  canonical imports.
- `opi-agent` and `opi-coding-agent` tests compile against the new path.
- README/examples and the actual `.crate` consumer use the canonical API path
  or the selected root facade; no `models::ChatApi` path remains.
- `cargo fmt`, documentation checks, `git diff --check` and the required
  package consumer are rerun after the source migration.

The chat portion is complete. Deferred contracts were intentionally finished
as the follow-up #166 slice so the entire operation-contract surface now has
one canonical owner.

## Test impact

`add`: this migration brief and canonical contract module.  
`update`: adapter, registration, preparation, workspace consumer and package
consumer imports.  
`delete`: the `models.rs` chat contract definitions and old `models::ChatApi`
path.  
`retain`: all behavior assertions and execution tests; no protocol semantics
are intentionally changed.
