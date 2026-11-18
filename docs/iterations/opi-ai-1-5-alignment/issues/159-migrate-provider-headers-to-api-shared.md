# #159 Move provider header policy to the canonical API shared namespace

Status: implemented; canonical-only namespace enforced
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Remove the remaining provider-header root module and make the shared wire
boundary explicit:

- `opi_ai::api::shared::headers` owns provider/model/request header validation,
  protected-name rules and deterministic source merging;
- `opi_ai::providers::*` continues to own provider identity, endpoint and
  catalog composition, while concrete `opi_ai::api::*` adapters consume the
  shared header contract;
- `opi_ai::ProviderHeaders` remains a selected crate facade, but there is no
  `opi_ai::provider_headers` module or duplicate implementation path.

## Ownership decision

`ProviderHeaders` is a protocol-neutral wire contract. It does not choose a
provider, endpoint or catalog, and it does not serialize a provider-specific
payload. The same validation and merge policy is consumed by every HTTP wire
adapter, classifier/image adapter, and mapped provider route. Keeping it at
the crate root made a cross-adapter wire seam appear to be a provider family;
`api::shared::headers` gives the behavior one deep owner while the root facade
continues to serve ordinary callers.

The lower-level syntax/error values remain in `types::headers`; the shared API
module owns the stateful validated value and merge policy. Provider profiles
and wire adapters now import the canonical path directly.

## Acceptance

1. `src/provider_headers.rs` is deleted and the implementation lives only at
   `src/api/shared/headers.rs`.
2. `api::shared` is the canonical public namespace for the header contract;
   `lib.rs`, adapters, providers, coding-agent callers, tests and utilities
   docs use that path. The root `ProviderHeaders` re-export is a facade, not a
   compatibility module.
3. No `provider_headers` module declaration or old module-path reference
   remains in active source, tests, docs or conformance scripts.
4. Header validation, reserved-name protection, merge order, callback output
   validation and provider request behavior retain their existing semantics.
5. A consumer built from the actual `.crate` archive resolves
   `opi_ai::api::shared::headers` and executes the protocol-adapters contract.

## Implementation and verification

The former root module was moved to
[`src/api/shared/headers.rs`](../../../../crates/opi-ai/src/api/shared/headers.rs).
`api::shared` is public for this focused contract while protocol-specific
mappers remain crate-private. All active internal callers use the canonical
namespace; the selected root facade is retained for ordinary SDK imports.

The external archive consumer in
[`issue-159-protocol-adapters-consumer-20261117.json`](../evidence/issue-159-protocol-adapters-consumer-20261117.json)
compiles and runs a direct `api::shared::headers` call from the extracted
package. The protocol-adapters runner fixture was also aligned with the
current expanded provider-owner feature union so its rejection and admission
tests validate the same graph as the archive consumer.

Verification completed:

- builtins library check passed;
- the minimum `--no-default-features` library check and rustdoc profile passed;
- selected `opi-ai` regression binaries passed (`17 + 11 + 111 + 54 + 2`);
- the follow-up OAuth wire-shape regression target passed (`12`);
- coding-agent provider-factory tests passed (`58`);
- warnings-denied Clippy for `opi-ai` and `opi-coding-agent` passed;
- provider canonical conformance passed (`23` tests) and protocol-adapters
  runner tests passed (`3` tests);
- formatting, documentation, Python compilation and whitespace checks passed;
- warnings-denied rustdoc for the builtins profile passed;
- the actual archive protocol-adapters consumer passed metadata, build and run,
  including the canonical shared-header call.

## Test impact

`add`: canonical namespace/root-deletion assertions and archive-consumer
source coverage.

`update`: header callers, public utility docs, module baseline and protocol
consumer.

`delete`: `src/provider_headers.rs`, its root declaration and old module path.

`retain`: header validation, merge ordering, protected-header checks and all
provider wire behavior.
