# Align the standalone telemetry README with AI-owned instrumentation

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Publish the feature profile matrix](30-readme-feature-profile-matrix.md)

## Scope

The standalone telemetry README still described AI request spans as a future
migration. That was stale after `opi-ai` gained its explicit request and poll
observers and could mislead users deciding whether the crates were actually
independent.

## Implementation

The English and Chinese `opi-telemetry` READMEs now state the current boundary:
explicit context lets ordinary `Models` chat install `pi.ai.request` and
deferred fetch install `pi.ai.poll`; raw stream/prepared attempts remain manual,
selected non-chat operations carry context without an AI span, and AI schema,
token, and cost semantics remain owned by `opi-ai`.

## Acceptance

`test_telemetry_readmes_describe_current_ai_integration_boundary` checks both
localized manuals. The packaged proof
[`phase4-telemetry-readme-ai-boundary.json`](../evidence/phase4-telemetry-readme-ai-boundary.json)
packages both crates and runs an external consumer in the two reviewed feature
graphs.

## Limits

This updates documentation and its contract test only. It does not broaden
automatic instrumentation or move AI schema into the neutral crate.
