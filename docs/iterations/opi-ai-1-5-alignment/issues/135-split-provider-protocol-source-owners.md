# #135 Split per-provider protocol/source owners

Status: completed for the independently selectable provider protocol/source
owners. Pair/high-order capability V is covered by #136; native macOS and #94
credential-source families remain open.

## Decision

`protocol-adapters` remains the compatibility group. The concrete protocol
modules now have independent public feature owners: `provider-anthropic`,
`provider-azure-openai`, `provider-bedrock`, `provider-gemini`,
`provider-mistral`, `provider-openai-chat`, `provider-openai-codex-responses`,
`provider-openai-responses`, `provider-openrouter`, `provider-pi-messages` and
`provider-vertex`. `provider-protocol-common` contains shared protocol seams and
`provider-openrouter-usage` contains shared usage accounting; neither is a
consumer-facing provider route. Bedrock and Vertex retain transitive
`cloud-auth`; Codex Responses retains `websocket-compression`. Default and
`builtins` continue to select the complete historical surface.

## Evidence

- Eleven Windows native owner checks, public profile tests and warnings-denied
  Clippy runs pass. The current compatibility `protocol-adapters` profile plus
  explicit `builtins` HTTP profile and acceptance tests also pass.
- The actual `.crate` was consumed outside the workspace for all eleven owners.
  The first ten owners pass in the original run; the first Vertex build failed
  only because the E-drive target filled, and the immutable v2 single-owner
  rerun passes after the authorized cache eviction.
- The caller inventory scans 1238 repository files and records direct module
  references, provider collection/factory callers, tests, scripts and docs with
  source hashes.
- Exact cache eviction plans and their receipts remain retained; no
  historical Pi 1.5.0 or earlier capability/protocol receipt was rewritten.

Evidence: [combined receipt](../evidence/phase8-provider-protocol-20261116.json),
[capability V matrix](../evidence/phase8-capability-v-20261116-v9.json),
[package run with the retained Vertex failure](../evidence/phase8-provider-protocol-package-20261116-v1.json),
[current package owner receipts](../evidence/phase8-provider-protocol-package-20261116-current.json),
[Vertex historical rerun](../evidence/phase8-provider-protocol-package-20261116-v2-vertex.json),
[current caller inventory](../evidence/phase8-provider-protocol-callers-20261116-v4.json).

Test impact: add the provider owner features, profile test, package runner/unit
tests, caller inventory and bilingual ownership documentation; update the
compatibility feature union and capability V matrix; delete none; retain all
prior Pi, adapter/observer, HTTP, protocol, cloud-auth and cache-maintenance
receipts.
