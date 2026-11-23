# Testing and Package Consumption

This page separates tests a downstream user can run from checks that maintainers
use to prove the packaged crate. It is not a runtime API reference.

## Faux Provider for External Tests

Use `test_support::MockProvider` and `test_support::single_route_collection` when a
consumer needs deterministic responses without credentials or a live service.
Script the final message, stream events, tool calls, delayed/pending results and
provider failures explicitly. The test owns the response script and the
cancellation token; the SDK still owns request construction, event ordering
and typed terminal settlement.

Use `test_support::ScriptedDeferredBackend` to share a scripted submission with
the explicit fetch and cancel operations. These Faux interfaces are available
in the minimum configuration. The UUID sequence boundary is exercised in the
`identity` module test. Scripted AWS source transports live in the Bedrock
fixtures.

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
Cargo consumer, verifies default and explicit `builtins` without tracing profiles,
checks that setup performs no source/store/request IO, and executes the exact
README examples. It records a JSON verdict under
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`.

For the minimum, run `scripts/pi-sdk-conformance/run_minimal_package.py` with
fresh `--scratch`, the shared `--target-dir` and a new `--evidence-output` path.
It verifies the actual archive graph with tracing off/on, executes core Faux
tool continuation, deferred submission and fetch, custom login Store ACK and
argument/numeric contracts. It checks that the old UUID sequence probe fails to
import from the archive and that backend dependencies are absent. This is a core profile proof, not a
claim that every Pi Faux behavior or supported feature/platform row is closed.
Use `--evidence-output` with the README runner as well to preserve prior receipts.

For the shared HTTP transport profile, run
`scripts/pi-sdk-conformance/run_http_transport_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["http-transport"]`, checks the transport-only dependency graph,
and executes the public pool, proxy and redirect contract outside the workspace.

For the TLS backend profile, run
`scripts/pi-sdk-conformance/run_tls_backend_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["tls-backend"]`, verifies the exact rustls/AWS-LC/native-certs
closure with reqwest and provider owners absent, and executes the public
`opi_ai::tls` trust/ALPN contract outside the workspace. The HTTP transport
profile separately proves that `http-transport` still selects this backend.

For the WebSocket/compression profile, run
`scripts/pi-sdk-conformance/run_websocket_compression_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["websocket-compression"]`, verifies the exact HTTP/TLS/PMD
dependency closure without provider adapters, and executes a workspace-external
HTTP/1.1 Upgrade with negotiated permessage-deflate. The native profile also
runs the public Upgrade/WSS contract and the private frame/PMD regression suite.


For the cloud authentication profile, run
`scripts/pi-sdk-conformance/run_cloud_auth_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["cloud-auth"]`, verifies the exact HTTP/TLS plus AWS/GCP
source/signing codec closure without provider protocol or compression owners,
and executes the public AWS endpoint, SigV4 and GCP ADC constants outside the
workspace.

For the provider protocol adapter profile, run
`scripts/pi-sdk-conformance/run_protocol_adapters_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["protocol-adapters"]`, checks the provider protocol dependency
closure including the transitive `cloud-auth`/`http-transport`/`tls-backend` owners without
the bundled catalog, and executes direct adapter, auth-store and WebSocket
surface calls outside the workspace. Provider collection and
built-in OAuth/AWS/Vertex credential factories remain a `builtins` proof.

For the provider OAuth/store profile, run
`scripts/pi-sdk-conformance/run_oauth_store_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It consumes the actual archive with `default-features = false,
features = ["oauth-store"]`, verifies the exact `http-transport` union,
rejects the bundled catalog and protocol/cloud-signing owners, and executes
the provider OAuth plus StoreOAuthResolver surface outside the workspace.
Neutral credential, mutation, store and OAuth contracts remain covered by the
minimum profile; provider collection and platform credential-source factories
remain a `builtins` proof.

For the per-provider protocol/source owners, run
`scripts/pi-sdk-conformance/run_provider_protocol_package.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
The runner consumes the actual archive outside the workspace and validates each
of these eleven owner features independently: `provider-anthropic`,
`provider-azure-openai`, `provider-bedrock`, `provider-gemini`,
`provider-mistral`, `provider-openai-chat`, `provider-openai-codex-responses`,
`provider-openai-responses`, `provider-openrouter`, `provider-pi-messages` and
`provider-vertex`. Use `--owner provider-vertex` to rerun one owner after a
transient package-build failure without rewriting the earlier receipt. Native
profile tests and warnings-denied Clippy use the same feature spelling.


For provider-owner pair and high-order capability V, run
`scripts/pi-sdk-conformance/run_provider_protocol_matrix.py` with a fresh
`--scratch`, the marked workspace `--target-dir` and a new `--evidence-output`.
It verifies all 55 unordered provider-owner pairs and the two supported
high-order unions natively, then consumes the actual archive from external
Cargo callers for the same rows. The receipt keeps the frozen Pi 1.5.0
paired-differential references and records native macOS as blocked when no
runner is available.

For the remaining shared-owner pairs, run
scripts/pi-sdk-conformance/run_shared_owner_pairs.py with a fresh
--scratch, the marked workspace --target-dir and a new --evidence-output.
It verifies cloud-auth+catalog-bundled and websocket-compression+oauth-store
with their owner profile tests, then consumes the actual archive from external
callers and checks the exact feature closure plus forbidden-owner boundaries.
Use --only to rerun one pair without rewriting the other receipt. The receipt
retains frozen Pi 1.5.0 references and records native macOS as blocked when no
runner exists.

## Development Checks

For a documentation-only change, run the conformance suite, documentation
contract checker, Rust formatter check, diff whitespace check, package file
listing and the packaged consumer proof. Live provider calls, fuzzing and
performance runs require their own explicitly enabled acceptance scope.
