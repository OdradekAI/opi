# #145 Split the Azure OpenAI Provider profile from its wire adapters

Status: implemented; canonical namespaces enforced; baseline verification caveat recorded below
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Apply the OpenRouter migration pattern to Azure OpenAI while making the
Provider/API boundary explicit. The Azure endpoint, deployment resolution,
catalog records and Provider construction belong under
`crates/opi-ai/src/providers/azure_openai.rs`. Azure Chat/Responses transport,
native options, stream mapping and the typed chat adapter belong under
`crates/opi-ai/src/api/azure_openai.rs`.

Remove the obsolete `opi_ai::azure_openai` root module during the 0.x path
migration. The canonical paths become:

- `opi_ai::providers::azure_openai` for the Provider profile;
- `opi_ai::api::azure_openai` for Azure wire adapters and native options.

## Ownership boundary

- The Provider profile owns endpoint/resource/deployment/api-version
  resolution, model records, shared OpenAI Chat composition and immutable
  dispatch configuration.
- The API adapter owns Azure `api-key` transport, Chat and Responses URL/body
  assembly, status mapping, SSE decoding, callbacks and
  `AzureOpenAIResponsesChatApi`.
- Shared OpenAI Chat/Responses mappers remain shared protocol owners; Azure
  only supplies its endpoint/auth/profile-specific composition.
- No request, auth, catalog, retry, stream terminal or native-option behavior
  is intended to change.

## Acceptance

1. Provider profile implementation lives under `src/providers/azure_openai.rs`.
2. Azure wire adapter and native options live under `src/api/azure_openai.rs`.
3. The obsolete `opi_ai::azure_openai` module and file are removed; no
   compatibility shim is retained for the migrated namespace.
4. Existing Azure fixtures, model/wire metadata, provider collection,
   transcript/tool serialization and coding-agent wiring remain green.
5. Minimal `provider-azure-openai` feature compilation proves both canonical
   modules; the external package consumer calls canonical Provider/API paths
   from the actual crate archive.
6. Documentation and task-owned whitespace checks pass. Any carried-in
   baseline failures are recorded with their exact source and are not folded
   into this migration.

## Work and verification

- Add this issue before changing the module ownership.
- Split the existing file with the smallest behavior-preserving delegation;
  avoid moving shared OpenAI protocol code into the Azure profile.
- Extend the provider profile probe and the existing package consumer instead
  of adding a parallel test harness.
- Record package-consumer evidence under the iteration evidence directory.

Completed in this slice:

- `crates/opi-ai/src/providers/azure_openai.rs` owns the Azure profile,
  endpoint/resource/deployment/api-version resolution, catalog records and
  immutable dispatch configuration.
- `crates/opi-ai/src/api/azure_openai.rs` owns the Azure Chat/Responses HTTP
  transport, native options, status mapping, SSE decoding and typed Responses
  adapter. The obsolete root module and file are removed.
- The provider profile probe and package consumer now call canonical Provider
  and API paths only. The obsolete root module is removed and all in-repository
  callers use canonical ownership paths.
- The external proof is recorded in
  [`issue-145-azure-openai-package-consumer-20261117-r3.json`](../evidence/issue-145-azure-openai-package-consumer-20261117-r3.json).

Verification results:

- PASS: `cargo fmt --all -- --check` and `python scripts/opi-doc-check.py`.
- PASS: minimal `--no-default-features --features provider-azure-openai`
  profile test (1 test).
- PASS: Azure fixtures (46), model/wire metadata (24), provider collection
  (111), transcript contract (29), auth boundary (13), service-tier cost (8),
  protocol adapter profile (2), and coding-agent Azure wiring (9).
- PASS: both tool-result provider behavior tests, including the public-module
  freeze. The intentional module-set update removes the obsolete Azure and
  OpenRouter root modules and records the existing public `tls` boundary.
- PASS: warnings-denied Clippy for `opi-ai` with the already recorded
  `-A clippy::too_many_arguments` baseline exception; no new warning comes
  from the split.
- PASS: provider protocol package consumer from the actual crate archive. It
  compiled and ran the canonical Provider/API paths with the expected Azure
  feature closure.
- PASS: provider package harness unit tests (6 tests) and task-owned
  whitespace checks.
- PASS: task-owned `git diff --check`. Repository-wide diff checking still
  contains unrelated carried-in CRLF whitespace and is not attributed to this
  migration.
- BASELINE BLOCKED: the broader protocol-adapters package runner stops at its
  existing feature-union assertion before building its consumer; its receipt is
  retained as `issue-145-protocol-adapters-package-consumer-20261117.json` for
  that separate harness repair.

## Test impact

`add`: canonical Provider/API path assertions and the package probe;
`update`: Azure module declarations and provider-owner consumer source;
`delete`: obsolete root modules; `retain`: existing Azure fixtures, model/wire,
provider collection, transcript, tool, coding-agent and historical Pi tests.
