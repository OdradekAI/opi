# Bound AI telemetry error status messages in UTF-16 units

Status: done  
Owner: Codex for user  
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)  
Depends on: [Ordinary chat telemetry starts at construction](20-ordinary-chat-telemetry-construction.md)

## Scope

Pi bounds `provider_error` status text with `ownedSlice(message, 0, 500)`.
JavaScript string length is measured in UTF-16 code units. Opi's previous Rust
helper counted Unicode scalar values, so a message made of non-BMP characters
could exceed the Pi limit.

## Decision and implementation

- count each Rust scalar with `char::len_utf16()`;
- stop before a scalar would cross 500 units;
- keep the result as an owned Rust `String`, which avoids retaining a large
  provider error body;
- do not manufacture an isolated surrogate, because Rust `String` cannot
  represent one. This is the explicit Rust-native boundary for the rare case
  where Pi's JavaScript slice would split a surrogate pair.

## Acceptance

- a 251-emoji error is recorded as exactly 250 emojis (500 UTF-16 units);
- the existing provider error status remains `provider_error` and is bounded;
- ordinary telemetry, setup failure, cancellation and package proofs remain
  unaffected.

## Evidence and source anchors

- Pi source: `.pi/pi-1.5.0/packages/ai/src/telemetry.ts:181-207` and
  `.pi/pi-1.5.0/packages/ai/src/utils/text.ts:45-50`.
- Rust implementation: `crates/opi-ai/src/telemetry.rs::owned_slice`.
- Regression: `crates/opi-ai/tests/ai_telemetry.rs::error_status_message_cap_counts_utf16_units`.

## Limits

This slice does not claim byte-for-byte equality for a JavaScript slice that
ends on an isolated surrogate. Live provider, full TS/Rust differential,
feature/platform/MSRV, performance and six-gate release acceptance remain
separate work.
