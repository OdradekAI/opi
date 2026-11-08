# Testing and Package Consumption

This page separates tests a downstream user can run from checks that maintainers
use to prove the packaged crate. It is not a runtime API reference.

## Faux Provider for External Tests

Use `test_support::MockProvider` and the public Faux registration helpers when a
consumer needs deterministic responses without credentials or a live service.
Script the final message, stream events, tool calls, delayed/pending results and
provider failures explicitly. The test owns the response script and the
cancellation token; the SDK still owns request construction, event ordering
and typed terminal settlement.

Faux is suitable for unit and integration tests of caller behavior. It does not
prove a real provider's authentication, wire serialization, rate limits or
service availability. Add a controlled peer or provider-specific fixture when
those boundaries are part of the behavior under test.

## Controlled Provider and Protocol Fixtures

The repository's controlled peers record requests, headers, source reads and
response order without contacting a live provider. A fixture should assert the
observable boundary it owns: request body, authentication source, retry count,
stream terminal event, cancellation phase or cleanup acknowledgement.

Protocol fixtures are complementary to module tests. Keep a focused unit test
for a pure parser or state machine, and a controlled peer test for the wiring,
ordering and side effects that the unit cannot observe.

## Test Layers and Replacement Rules

- Module tests cover private parsers, state machines and invariants that no
  external caller can observe.
- Public contract tests cover values, options, errors, features and rustdoc
  examples at the public boundary.
- Protocol/consumer tests cover request construction, stream events, auth reads,
  cancellation, Store publication and cross-module ownership.
- Package proof consumes the actual `.crate` outside the workspace and runs the
  three README onboarding programs in both feature profiles.

Remove a test only after its input class, timing, error stage, side effect and
resource risk remain detected by a named replacement. A full integration test
may replace a trivial delegation unit test, but it cannot replace a unique
public-value or parser boundary assertion.

## Packaged Consumer Proof

Run the offline proof from the repository root:

```text
python -B scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch C:/Temp/opi-readme-proof-20261108-restructure --target-dir E:/opi/cargo-targets/opi-72fa37a0fa044958
```

The proof packages `opi-telemetry` and `opi-ai`, creates an ordinary external
Cargo consumer, verifies both default and `default-features = false` profiles,
checks that setup performs no source/store/request IO, and executes the exact
README examples. It records a JSON verdict under
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`.

## Development Checks

For a documentation-only change, run the conformance suite, documentation
contract checker, Rust formatter check, diff whitespace check, package file
listing and the packaged consumer proof. Live provider calls, fuzzing and
performance runs require their own explicitly enabled acceptance scope.
