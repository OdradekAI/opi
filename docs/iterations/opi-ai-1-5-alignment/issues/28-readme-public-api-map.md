# Publish the public API map before the reference chapters

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Add task-oriented README navigation](27-readme-task-guide.md)

## Scope

An independent consumer should be able to identify the stable ownership and
execution boundaries without searching provider-specific chapters. The README
needs a compact map of the public driver, request values, stream contract,
context/tools, and telemetry seam.

## Implementation

Both localized READMEs now include an API-at-a-glance table immediately before
Quick Start. It names the public entry points and states the responsibility of
each boundary; provider adapters are documented as implementations behind the
neutral entries.

## Acceptance

`test_readmes_publish_public_api_at_a_glance` requires the same six public
surface entries in English and Chinese. The packaged proof continues to compile
and execute the linked complete examples.

## Limits

This is a navigation and ownership map. It does not replace rustdoc signatures,
provider protocol references, or the detailed acceptance records below it.
