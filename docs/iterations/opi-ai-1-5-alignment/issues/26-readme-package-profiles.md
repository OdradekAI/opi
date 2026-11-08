# Exercise README onboarding in both public feature profiles

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Align pricing tiers, cache pricing, and provider-reported cost](25-cost-pricing-semantics.md)

## Scope

An independent `opi-ai` consumer must be able to follow the README without
`opi-agent`, the CLI, workspace source paths, or hidden global initialization.
The same packaged onboarding sources must compile and execute with the
documented default build and with `default-features = false`.

## Implementation

- `consumer-readme/Cargo.toml` provides a no-default template; the runner
  derives the default profile by removing that override in the external copy.
- `run_readme_examples.py` executes metadata, compilation, a controlled-peer
  regression, and all three README flows in both feature profiles.
- Profile metadata must show no `telemetry-tracing` in `defaults-off` and the
  default `telemetry-tracing` feature in `defaults-on`; both profiles must resolve the
  extracted `.crate` artifact rather than workspace source.
- The onboarding source commands use `cargo check --example ...` and
  `cargo run --example ...` after copying a source into the consumer's
  `examples/` directory; they do not require `-p opi-ai` or a workspace member.
- The English and Chinese README explain the two profiles and point to the
  offline package proof. The exact three example sources remain embedded in
  both manuals and are checked byte-for-byte against the packaged artifact.
- Both manuals publish a complete standalone consumer `Cargo.toml` for the
  three onboarding examples, plus the `default-features = false` replacement,
  so an external developer does not need to guess runtime dependencies.

## Acceptance

The proof requires both profiles to pass setup with zero source/store I/O,
execution before consumer polling, the final response/stream behavior, and the
tool-result continuation. The external graph must not contain `opi-agent`,
`opi-coding-agent`, `opi-protocol`, or `opi-tui`.

Evidence: `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`.
The recorded run used the extracted `opi-ai-0.8.3.crate`, with
`defaults-off` resolving no opi-ai features and `defaults-on` resolving the
default `telemetry-tracing`; every step exited zero.

## Limits

This is a packaged onboarding and feature-profile proof, not a claim that every
provider path or the complete pi-coding-agent surface is aligned. Live provider
calls, publication, MSRV, and full release acceptance remain separate gates.
