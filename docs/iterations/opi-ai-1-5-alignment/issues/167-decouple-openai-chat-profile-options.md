# #167 Decouple OpenAI Chat profile from native wire options

Status: complete  
Owner: current implementation slice  
Parent: [#164 unresolved ownership audit](164-audit-unresolved-ownership-boundaries.md)

## Goal

Make the first provider/profile boundary concrete. The OpenAI Chat profile
keeps identity, endpoint, catalog, compatibility policy, headers and client
state. `OpenAiCompletionsOptions` belongs to the API wire adapter and is
carried only for the current dispatch.

## Evidence and scope

`OpenAiChatProvider::native_options` is written only by the API module while
building a simple or direct native call, and is read only by the OpenAI Chat
request-body projection. No public profile constructor or consumer needs that
state. The ordinary Provider path has no native options and can use the same
projection with an explicit `None`.

This slice therefore:

- removes the API-native field and API import from
  `providers::openai::OpenAiChatProvider`;
- passes an owned optional native option value through the OpenAI wire
  projection for simple/direct calls;
- preserves the public `build_request_body` behavior and all retry/resource
  semantics;
- adds an archive structural assertion that the OpenAI profile does not store
  native options.

It does not yet remove the adapter's dependency on the profile type. That is a
separate route-context extraction step and remains open in #164.

## Acceptance

- `providers::openai` imports no `api::openai::OpenAiCompletionsOptions` and
  contains no `native_options` field.
- OpenAI Chat fixtures and external SDK tests preserve wire output.
- `opi-agent` and `opi-coding-agent` compile.
- The provider protocol package consumer verifies the profile/source layout.
- No compatibility shim or duplicate provider root is introduced.

Observed on 2026-11-17:

- `cargo check -p opi-ai --locked --features builtins` passed.
- `openai_chat_fixtures`: 116 passed.
- `external_sdk`: 11 passed.
- `opi-agent` and `opi-coding-agent` package checks passed.
- The archive protocol-adapters consumer passed with the OpenAI profile
  layout assertion enabled.
- The independent `provider-openai-chat` archive consumer passed with the
  same canonical operation/profile assertions.

## Test impact

`update`: OpenAI Chat wire fixtures and package structural consumer.  
`delete`: the profile-owned native option field and its mutation path.  
`retain`: all request serialization, auth, retry, callback, resource and
telemetry behavior.
