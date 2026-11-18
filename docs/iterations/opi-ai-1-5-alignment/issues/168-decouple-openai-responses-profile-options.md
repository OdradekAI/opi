# #168 Decouple OpenAI Responses profile from native wire options

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)  
Precedent: [#167 OpenAI Chat profile options](167-decouple-openai-chat-profile-options.md)

## Evidence

`providers::openai_responses::OpenAiResponsesProvider` previously imported
`api::openai_responses::OpenAiResponsesOptions` and stored `native_options`.
The API module read that state while compiling reasoning, summary and
service-tier fields, and the direct Chat API wrote it during dispatch. This
was the same ownership violation removed from OpenAI Chat in #167, with an
additional service-tier projection that must stay in the API adapter.

## Implemented seam

`ResponsesConfig`, endpoint/catalog/auth policy, affinity policy and client
state remain in the provider profile. The API request compiler and stream
producer now receive an explicit optional `OpenAiResponsesOptions`: the
ordinary Provider path passes `None`, while simple/native entries pass the
current owned options. Azure's shared Responses wire passes its projected
options explicitly. Profile cloning selects only the model/catalog route and
never carries wire-native state. Prepared-signin recursion and the deferred
API route remain intact.

## Acceptance

- `providers/openai_responses.rs` contains no API-native option import or
  `native_options` field.
- Responses fixtures, deferred lifecycle and external SDK tests pass.
- A single-provider archive consumer proves the independent feature closure and
  canonical profile layout.
- No profile shim or duplicate root module is introduced.

Observed on 2026-11-18:

- `cargo check -p opi-ai --locked --features builtins` passed.
- `cargo check --workspace --locked` passed; the provider profile integration target passed (1 test).
- `azure_openai_fixtures`: 46 passed.
- `openai_responses_fixtures`: 84 passed.
- `openai_responses_lifecycle`: 15 passed.
- `external_sdk`: 11 passed.
- Canonical package layout checks and the provider protocol layout unit suite:
  30 passed; `opi-doc-check.py` and `git diff --check` passed.
- The independent `provider-openai-responses` consumer built and ran from the
  extracted `opi-ai-0.8.3.crate`; the recorded verdict was `PASS` in
  `C:\Temp\opi-openai-responses-profile-20261118-r1\evidence.json`.
- The protocol-adapters archive consumer also built and ran from the extracted
  crate; its recorded verdict was `PASS` in
  `C:\Temp\opi-canonical-contract-20261118-r1\evidence.json`.

## Test impact

`update`: OpenAI Responses and Azure Responses fixtures, lifecycle tests and
package structural consumers.  
`delete`: the OpenAI Responses profile-owned native option field, its API import
and its mutation path.  
`retain`: all request serialization, auth, retry, callback, deferred,
resource and telemetry behavior.
