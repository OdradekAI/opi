# #160 Move provider environment lookup to the canonical API shared namespace

Status: implemented; canonical-only namespace enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining provider-environment root module and make the shared API
boundary explicit:

- `opi_ai::api::shared::env` owns scoped provider-environment lookup;
- provider profiles remain responsible for provider identity, endpoints,
  credentials and catalogs;
- API adapters may consume the neutral lookup contract without importing a
  provider profile or a protocol-specific implementation;
- there is no `opi_ai::provider_env` compatibility module or duplicate path.

## Ownership decision

`provider_env_value` only resolves a named value from a caller-supplied scope
and, when needed, the ambient process environment. It performs no provider
selection, endpoint construction, authentication, proxy policy, caching or
wire serialization. Its actual consumer is the Pi Messages API adapter's
`PI_CACHE_RETENTION` option, and the same deterministic lookup contract is
usable by other API adapters. The root placement therefore overstated provider
ownership and made a shared API concern appear alongside the public Provider
contract.

`api::shared::env` is the canonical owner because it is a public, protocol-
neutral adapter utility beside the existing shared header contract. The
provider profile namespace remains reserved for identity and composition; no
`providers::shared` namespace is introduced for a helper with no profile or
catalog state.

## Acceptance

1. `src/provider_env.rs` is deleted and the implementation lives only at
   `src/api/shared/env.rs`.
2. `api::shared::env` is the canonical public path; internal callers, tests,
   docs and the external consumer use it directly.
3. `lib.rs` no longer declares or re-exports `provider_env`.
4. Scoped-first lookup, empty-value fallback, ambient lookup and no-mutation
   behavior remain covered by the existing public utility tests.
5. A consumer built from the actual `.crate` archive resolves and executes
   `api::shared::env` under the protocol-adapters feature.

## Implementation and verification

The former root module was moved to
[`src/api/shared/env.rs`](../../../../crates/opi-ai/src/api/shared/env.rs).
`api::shared` now exposes both the shared header and environment contracts;
the Pi Messages adapter and public utility tests import the canonical path.
The module-freeze baseline was updated to remove the deleted root declaration,
and the conformance suite asserts both root deletion and canonical callers.

The external protocol-adapters consumer now calls
`opi_ai::api::shared::env::provider_env_value` from the extracted archive. Its
metadata, build and run receipt is recorded in
[`issue-160-protocol-adapters-consumer-20261117.json`](../evidence/issue-160-protocol-adapters-consumer-20261117.json)
(`PASS`; all nine steps exited zero).

The implementation checks also passed: builtins and no-default library checks;
`chat_pi_messages` (42), `external_sdk` (11), `public_utilities` (54) and
`tool_result_wire` (2); warnings-denied Clippy for `opi-ai` and
`opi-coding-agent`; builtins and no-default warnings-denied rustdoc; canonical
conformance (25) and protocol runner tests (3); Python compilation; formatting,
documentation and whitespace checks.

## Test impact

`add`: canonical namespace/root-deletion assertions and archive-consumer source
coverage.

`update`: Pi Messages adapter import, public utility imports, module baseline,
CHANGELOG and iteration documentation.

`delete`: `src/provider_env.rs` and its root module declaration.

`retain`: scoped environment behavior tests and Pi Messages cache-retention
wire tests.
