# #146 Split the Anthropic Provider profile from the Messages wire adapter

Status: implemented; canonical namespaces enforced; baseline verification caveat recorded below
Owner: current implementation slice
Parent: [opi-ai 1.5.0 alignment spec](../spec.md)

## Goal

Move the Anthropic service profile and its Messages protocol implementation to
the responsibility-oriented layout without retaining `opi_ai::anthropic` as a
compatibility module. The canonical paths become:

- `opi_ai::providers::anthropic` for the Provider profile and model catalog;
- `opi_ai::api::anthropic` for Anthropic Messages options, SSE/event mapping,
  request serialization, transport and typed chat adapter.

## Ownership boundary

- The Provider profile owns provider identity, default endpoint, model catalog,
  route headers, auth-invalid policy, Copilot header mode, shared client and
  per-dispatch native/profile state.
- The API adapter owns Anthropic SSE framing and event types, usage mapping,
  tool/thinking/cache request projection, HTTP status/auth handling, streaming,
  `AnthropicMessagesOptions` and `AnthropicMessagesChatApi`.
- OpenRouter-specific accounting remains in the crate-private
  `crate::api::shared::openrouter_usage` wire owner; moving the Anthropic wire
  parser must not make that accounting part of the Anthropic provider profile.
- No request, auth, catalog, retry, stream terminal, telemetry or lifecycle
  behavior changes are intended.

## Acceptance

1. The profile implementation and catalog live under
   `src/providers/anthropic.rs`.
2. The Anthropic Messages wire implementation and native options live under
   `src/api/anthropic.rs`.
3. The obsolete `src/anthropic.rs` file and `opi_ai::anthropic` module are
   removed; no compatibility shim is retained.
4. All in-repository callers, tests, docs and package probes use canonical
   `providers::anthropic` and `api::anthropic` paths.
5. Existing Anthropic fixtures, lifecycle/auth/telemetry/replay/tool tests,
   coding-agent wiring, minimal feature compilation and an external consumer
   built from the actual crate archive pass.
6. Formatting, documentation and task-owned whitespace checks pass.

## Work and verification

- Add this issue before changing the module ownership.
- Split the existing file with the smallest behavior-preserving delegation;
  keep the profile responsible for identity/catalog/dispatch state and the API
  module responsible for the Anthropic Messages wire contract.
- Update internal callers, tests and package probes to canonical namespaces;
  remove the obsolete root module rather than retaining a compatibility shim.
- Record an external consumer proof built from the actual crate archive.

Completed in this slice:

- `crates/opi-ai/src/providers/anthropic.rs` now owns the Anthropic Provider
  profile, model catalog, headers, auth policy and dispatch configuration.
- `crates/opi-ai/src/api/anthropic.rs` now owns the Messages options, SSE/event
  parser and mapper, request serialization, transport/status handling and typed
  Chat adapter. The obsolete root module and file were removed.
- All active in-repository callers, tests, docs and package probes use
  `providers::anthropic` for the profile and `api::anthropic` for the wire
  adapter. Historical snapshots and prior evidence retain old paths only as
  audit records.
- The external proof is recorded in
  [`issue-146-anthropic-package-consumer-20261117-r1.json`](../evidence/issue-146-anthropic-package-consumer-20261117-r1.json).

Verification results:

- PASS: `cargo fmt --all -- --check`.
- PASS: minimal `--no-default-features --features provider-anthropic` profile
  test (1 test), confirming both canonical modules compile through the narrow
  feature closure.
- PASS: Anthropic parser/wire/stream/usage/EOF fixtures (176 tests total:
  `anthropic_fixtures` 155, `anthropic_thinking_roundtrip` 13,
  `anthropic_usage_updates` 5, `anthropic_nested_usage` 1,
  `anthropic_repeated_message_stop` 2).
- PASS: structure and consumer boundary tests (277 tests total across
  protocol adapter, provider collection/lifecycle/error/auth/enrichment,
  proxy/pooling/replay/telemetry/tool-result targets).
- PASS: `model_wire_metadata` (24 tests), including exact Anthropic catalog
  ownership and strict wire rejection through the canonical profile/API split.
- PASS: warnings-denied Clippy for `opi-ai` with the existing
  `-A clippy::too_many_arguments` baseline exception; the split adds no
  warning.
- PASS: coding-agent consumers (`provider_factory` 58,
  `provider_stream_diagnostics` 2, `anthropic_usage_recovery` 2,
  `oauth_auth` 124, `session_runtime` 71).
- PASS: canonical-only provider package consumer built and ran from the actual
  crate archive; its source references both `opi_ai::api::anthropic` and
  `opi_ai::providers::anthropic`.
- PASS: provider package harness tests, Python syntax checks, documentation
  checker and canonical source scans.
- PASS with the repository's existing CRLF convention: task-owned Rust/docs
  paths pass `git diff --check`; the carried-in protocol-adapter Python runner
  retains CRLF line endings, so its three canonicalized source lines are
  checked with `git -c core.whitespace=cr-at-eol diff --check`.
- BASELINE CAVEAT: the broader protocol-adapters package matrix retains its
  existing feature-union limitation; this Anthropic owner proof uses the
  owner-specific runner and passes independently.

## Test impact

`add`: canonical Provider/API profile assertions, canonical-only external
consumer source and the migration receipt; `update`: module declarations,
internal imports, current docs and tests; `delete`: obsolete root module/file;
`retain`: historical snapshots and evidence as audit records.
