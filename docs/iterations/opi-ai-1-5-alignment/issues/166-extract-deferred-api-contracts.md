# #166 Extract Deferred operation contracts into `api::contract`

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)

## Goal

Finish the operation-contract extraction started by #165. The Deferred
operation seam belongs to `api::contract`; the pure handle and error values
remain in `types::deferred`. Delete the root `deferred` module instead of
retaining a compatibility shim.

## Scope

- Move `DeferredFetchCall`, `DeferredCancelCall`, `DeferredApi` and
  `DeferredApiHandle` to `api::contract`.
- Keep `DeferredHandle`, `DeferredError` and `RouteMismatchDetail` in
  `types::deferred`.
- Update Models, Provider, protocol adapters, test support, workspace tests,
  `opi-agent` and `opi-coding-agent` to the canonical paths.
- Re-export the canonical contract from `opi_ai::api` and the selected crate
  root facade; remove `opi_ai::deferred` entirely.
- Preserve submission, fetch, cancellation, validation and telemetry
  behavior. This is an ownership move, not a runtime change.

## Canonical result

```text
opi_ai::api::contract::{
    DeferredApi, DeferredApiHandle, DeferredCancelCall, DeferredFetchCall,
}
opi_ai::types::deferred::{DeferredError, DeferredHandle, RouteMismatchDetail}
```

The root facade may re-export these names intentionally. No `deferred` module
or compatibility shim remains.

## Verification

- `cargo check -p opi-ai --locked --features builtins`
- Deferred contract, lifecycle, telemetry and external SDK tests
- `opi-agent` and `opi-coding-agent` compile/tests using canonical imports
- documentation checks, formatting and diff whitespace checks
- actual archive/package consumer probe after the workspace checks

Observed on 2026-11-17:

- `cargo check -p opi-ai --locked --features builtins` passed.
- `deferred_contract`: 19 passed; `external_sdk`: 11 passed.
- `python -B -m unittest discover -s scripts/pi-sdk-conformance -p
  'test_provider_protocol_package.py' -v`: 28 passed.
- `run_protocol_adapters_package.py` passed against the extracted archive and
  checked the canonical operation-contract layout in that archive.

## Test impact

`update`: Deferred contract, lifecycle, telemetry, agent and external SDK
imports; no behavior assertions change.  
`delete`: root `deferred.rs` and its old public module path.  
`retain`: value semantics, route validation, cancellation and telemetry
behavior.
