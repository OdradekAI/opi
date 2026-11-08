# Index the advanced README references

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Publish the public API map before the reference chapters](28-readme-public-api-map.md)

## Scope

The task-oriented guide and API map make the first call discoverable, but the
manual still contains long provider, protocol, cost, telemetry, deferred, and
test sections. Those sections need one explicit index so a consumer can jump to
an advanced capability without scanning the full README.

## Implementation

Both localized READMEs now include a compact reference map immediately before
Quick Start. It links provider and factory/auth contracts, credential refresh,
telemetry, cost semantics, the WebSocket upgrade bridge, deferred responses, and
the test/package proof. The map also states that a reference link does not imply
that every provider or transport is enabled in every feature profile.

## Acceptance

`test_readmes_index_advanced_capability_references` requires the same advanced
capability links in English and Chinese. The normal documentation checker and
the packaged consumer proof remain required after README edits.

## Limits

This slice improves navigation only. It does not move the long reference
chapters into separate files or change feature availability and runtime
behavior.
