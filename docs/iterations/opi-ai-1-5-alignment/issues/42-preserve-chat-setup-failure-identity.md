# Preserve request identity in chat setup failures

Status: done
Owner: Codex for user
Parent: [alignment specification](../spec.md)
Predecessor: [pair public Models setup failures](41-match-public-models-setup-failures.md)
Baseline: opi `ee18dd1323a089d536bdc7e293d0b4869777c344` with the #39–#41 worktree changes

## Behavior

Ordinary `Models` chat calls use the frozen `ChatModelRecord` to populate
`provider` and `model` when the driver synthesizes an error message during
setup. This covers route lookup, request validation, auth, preparation for
dispatch, and request-token cancellation returned during setup. A partial
attached to the cause takes precedence over the synthesized message.
The stream and `result()` deliver the same typed `CallFailure`.

The missing-provider regression deliberately gives `Request.model` an
unrelated value. The failure identifies the frozen record, carries unknown
usage, appears once, and settles the request span. Native preflight tests
also exercise a record whose provider differs from the adapter's registration
and a request cancelled before auth.

## Frozen replay

The manifest binds the original-lock candidate at
`C:/Temp/opi-ai-frozen-lock-20261111`. Source, root and installed lock files,
generated trees, catalog and dependency inspection match #41. Of the tracked
material inputs, the Rust source tree and lifecycle comparator have new hashes.

| Public setup case | Failure message provider | Failure message model | Rust cause |
| --- | --- | --- | --- |
| `models-missing-provider` | `missing-provider` | `mock-model` | `Config` |
| `models-auth-setup-failure` | `lifecycle-differential` | `mock-model` | `AuthFailed` |

Both runtimes matched those message IDs and delivered one failure followed by
EOF. All four cases passed their shared checks across 12 TypeScript and four
Rust invocations, with zero material blockers. Overall verdict
`DIFFERENCES_REQUIRE_DECISION` / exit 0.

On the explicit-key path, Pi entered the resolver before the call returned.
The current-thread Tokio probe entered it in the execution task after the
call returned and before the consumer polled. Pi delivered an error event and
fulfilled `result()` with the error message. Rust delivered `Err(CallFailure)`
from the stream and from `result()`. The timing and delivery decisions remain
open under the [public contract](../decisions/04-public-contract.md) and
[lifecycle policy](../decisions/11-telemetry-lifecycle.md).

## Verification

- [Material manifest](../evidence/phase5-setup-failure-identity-materials.json), SHA-256 `19ac70bbd6d1ea8e117f8a25efb5a7977c53cd90c5d7fb30401b365fd4341368`.
- [Replay commands and raw traces](../evidence/phase5-setup-failure-identity-differential.json), SHA-256 `1ed994a7fdc068db18891534e844c0c0d8d4e545286c6248cb0fd9e4fe340d06`.
- Before the fix, the public missing-route test failed on the empty provider ID.
  The updated comparator rejects both #41 Rust setup traces on their empty
  provider and model fields. The replay CLI rejects #41's stale manifest with
  exit 2 before launching either driver.
- Cached `test -p opi-ai --lib --offline -j4 stream::` passed 23 tests.
  Cached tests for `ai_telemetry`, `custom_provider_registration`,
  `execution_lifecycle`, `node4_preparation`, `resource_cleanup`, and the
  `lifecycle_differential` example passed 20, 35, 29, 26, 16, and two tests.
- Cached Clippy passed for the library, the two edited test targets, and the
  lifecycle example with `-D warnings`. The Python differential suite passed
  19 tests. Cargo format, the documentation checker, and diff checks passed.

Test impact `add` for attached-partial precedence and the comparator's
identity mismatch regression. `update` for the existing lifecycle and native
preflight assertions and comparator fixtures. `retain` for both source
completion cases and all previous evidence.

Worktree handoff `committed=none-for-this-task`, `staged=none`. The iteration
spec, contract index and differential README, comparator and comparator tests
were carried in and updated. This issue and its two JSON records are new
untracked paths. Verification covers the affected local behavior on Windows x64.
