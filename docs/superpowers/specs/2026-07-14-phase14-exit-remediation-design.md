# Phase 14 Exit Remediation Design

Status: user-approved on 2026-07-14; Phase F closure addendum approved and
superseded on 2026-07-17; pi-0.80.6 alignment revision user-approved on
2026-07-17.

This design responds to the Phase 14 phase-exit result `not-met-block`. It is
self-contained: the generated reports under
`target/opi-artifacts/phase14-task14.7/` are evidence for why remediation is
needed, but they are not required to reconstruct the corrective task graph.

## Context

Tasks 14.1 through 14.7 were implemented and committed, but the independent
phase-exit audit accepted 20 blocking findings. SC4 (Request and session
affinity) and SC7 (dynamic refresh substrate) were met. SC1, SC2, SC3, SC5,
SC6, SC8, and the `api-map` residual were not met. All eight Phase 14
Non-Goals remained respected.

The audit also reran all 29 recorded acceptance-scenario commands. Only 11
selected a test, 5 named nonexistent integration-test targets, and 13 exited
successfully while selecting zero tests. A successful command that runs no
test is not acceptance evidence.

The remediation goal is strict closure: preserve the shipped history of
14.1-14.7, add corrective tasks 14.8-14.13, close every accepted blocker, and
rerun phase exit until the phase is genuinely eligible for archive.

## Decisions

1. Add a separate corrective design rather than rewriting the historical
   Phase 14 design in place.
2. Register this file as a reviewed Phase 14 supplemental source before
   reinitializing the `opi-implement` task graph.
3. Preserve the existing commits, `passing` task history, evidence, and
   `verified_at_commit` values for 14.1-14.7. Reinit may repair their
   acceptance-command metadata because that field is explicitly
   reinit-editable, but it must not rewrite shipped history.
4. Implement native credential stores for all six release targets: Windows
   Credential Manager, macOS Keychain Services, and Linux Secret Service.
5. Follow the reviewed public API contract even though it is a 0.x breaking
   correction: `Usage` child fields become `Option<u64>`, the extra 1-hour
   cost line is removed, and malformed subsets become errors.
6. Historical decision, superseded by the 2026-07-17 alignment revision:
   reassess `api-map` without implementation. The reviewed pi comparison later
   proved that GitHub Copilot is the concrete multi-wire driver, so Phase 14
   now implements `api-map`.

## Goals

- Make persisted credentials usable through a real native store in normal
  product startup.
- Exercise doctor, model listing, login, logout, credential-needed retry, and
  non-interactive remediation through production orchestration rather than
  helper-only seams.
- Close the concrete Anthropic cache-marker wire path.
- Restore the reviewed Usage and cost contract, including strict malformed
  response handling and session resume.
- Make every public document, runtime help surface, acceptance command, and
  residual disposition truthful.
- Align provider identity, Codex login/wire behavior, and the GitHub Copilot
  multi-wire catalog with the reviewed pi 0.80.6 implementation.
- Provide the same typed model-to-wire contract to Rust consumers and TOML
  custom providers.
- Pass the full Phase 14 exit audit without weakening its criteria.

## Non-Goals

The original eight Phase 14 Non-Goals remain binding:

- no opi-managed plaintext credential file;
- no automatic re-login after mid-stream revocation;
- no per-call credential or provider-managed auth-header override;
- no `onPayload` or `onResponse` streaming hooks;
- no `maxRetries` or `maxRetryDelay` fields on `Request`;
- no end-to-end `SecretString` provider-construction migration;
- no OAuth providers beyond Anthropic, GitHub Copilot, and OpenAI Codex;
- no session-schema-version or context-reconstruction redesign.

The remediation still does not add a production model-refresh trigger or
weaken SC4/SC7. It now implements `api-map`; the prior deferral is explicitly
superseded below.

## Source and Ledger Integration

Before implementation starts:

1. Add this file to the Phase 14 supplemental-source registry in
   `.claude/skills/opi-implement/skill.md`.
2. Add a remediation pointer to the existing Phase 14 design and update the
   Phase 14 status in both `docs/opi-spec.md` and `docs/opi-spec.zh.md` to
   `implemented; remediation pending` (localized in the Chinese file).
3. Invoke the normal `opi-implement` spec-drift/reinit path. Do not hand-edit
   `.opi-impl-state.json`.
4. Rebuild the acceptance trace so the original tasks plus 14.8-14.13 jointly
   own SC1-SC8.
5. Repair the 29 existing acceptance commands during reviewed reinit. Preserve
   their scenario meaning and runtime history.

The current deletions of `scripts/*.workflow.js` and the untracked copies
under `.claude/skills/opi-implement/scripts/` are pre-existing user changes.
They remain baseline dirty files and are excluded from every corrective task's
staging and commit unless the user separately assigns that relocation.

## Task Graph

```text
14.8 native keyring and probes
  -> 14.9 login/logout dispatcher
    -> 14.10 live auth and session interaction

14.11 factory-built Anthropic cache markers ----+
14.12 Usage and cost contract ------------------+-> 14.13 final alignment
14.10 live auth and session interaction --------+       -> phase exit
```

Tasks 14.8-14.10 are sequential. Tasks 14.11 and 14.12 are logically
independent, but they must not run concurrently if their concrete
task-owned-file lists overlap. Task 14.13 starts only after 14.8-14.12 pass.

All six tasks set `evaluator_required = true`. Credential security, public API
changes, TUI terminal state, concrete provider wire behavior, and final phase
closure are all risk-gated work.

## 14.8 - Native Keyring and Production Probes

### Platform stores

Keep `keyring-core` as the abstract API and add target-specific production
store dependencies through workspace dependencies:

| Release targets | Store crate | Native service |
|---|---|---|
| `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | `windows-native-keyring-store` | Windows Credential Manager |
| `x86_64-apple-darwin`, `aarch64-apple-darwin` | `apple-native-keyring-store` keychain store | macOS Keychain Services |
| `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `zbus-secret-service-keyring-store` | Freedesktop Secret Service |

Production initialization creates the target store and calls
`keyring_core::set_default_store` before doctor, model listing, live provider
construction, or interactive startup can create an entry. A process-lifetime
guard releases the default store with `unset_default_store` on orderly
shutdown. Completion generation and commands that do not touch credentials do
not need to initialize a store.

The sample and mock stores bundled with `keyring-core` are forbidden in
production. Tests continue to inject fake backends and must never access the
user keychain. Local verification compiles and exercises the host selection;
the existing release build matrix remains the six-target compile check.

### Resolver semantics

Credential resolution distinguishes absence, expected platform
unavailability, corruption, and operational failure:

| Store result | API-key behavior | OAuth behavior |
|---|---|---|
| Valid stored credential | Use it | Use it |
| Entry absent | Query the configured environment variable | `CredentialNeeded` |
| Linux Secret Service unavailable/no daemon | Query env and retain a redacted backend-unavailable diagnostic | Fail: OAuth persistence requires a keychain |
| Malformed envelope or unknown version/type | Return the typed store error; no env fallback | Return the typed store error |
| Other backend operation failure | Return the typed store error; no env fallback | Return the typed store error |

This resolves the apparent tension between headless API-key fallback and the
rule that corrupted persisted state must never be hidden by an environment
variable.

### Doctor and model listing

The asynchronous doctor and model-listing entry points receive the production
credential service and call `probe` themselves. Synchronous formatting helpers
receive only redacted probe results.

`--list-models` builds a listing collection from configured model metadata and
redacted auth state. It does not construct a live provider or resolve an API
secret merely to list configured models. A stored-only credential must
therefore list models when the corresponding environment variable is absent.

Integration tests inject a fake backend into the same asynchronous command
orchestration called by production. Precomputed probe values are not accepted
as product-path evidence.

### Definition of done

- Every release target has an explicit native-store dependency and startup
  branch.
- Native initialization precedes all credential-aware product paths.
- Corrupt and unknown envelopes cannot fall through to env.
- Stored-only `--list-models` succeeds through production orchestration.
- Doctor and listing report `Present`, `Absent`, and `BackendUnavailable`
  without reading or printing a secret.
- Redaction/temp-root scans continue to prove that only the secret-free lock
  file exists outside the fake backend.

## 14.9 - Login/Logout Dispatcher and Persistence

Extract one production slash-command dispatcher used by the interactive TUI.
It receives injectable command services: the credential store, OAuth registry,
login presenter, and a terminal suspension guard. Tests call this dispatcher;
they do not reproduce its branching logic.

`/login <provider>` runs the selected production OAuth implementation and
writes the resulting credential through the same locked mutation coordinator
used by refresh. `/logout <provider>` deletes through that coordinator.
Anthropic and Codex retain PKCE callback/manual-code fallback behavior.
Copilot presents its device URL and code and polls the device authorization;
that presentation is its headless/manual flow and it does not request a
paste-back code.

Raw-mode and alternate-screen suspension use RAII. Success, failure,
cancellation, presenter failure, callback timeout, and lock contention all
restore the previous terminal state exactly once.

### Definition of done

- The real dispatcher completes Anthropic, Copilot, and Codex login through
  the production registry and persists into an injected locked store.
- Logout removes each stored profile through the same coordinator.
- Manual fallback, cancellation, lock failure, and terminal restoration are
  covered without a real browser, keychain, terminal, or provider network.
- A locked or unavailable store produces a typed, redacted failure and never
  reports login success.

## 14.10 - Live Auth and Session Interaction

### Lazy construction and per-stream resolution

The provider factory must be able to construct the approved Anthropic,
Copilot Chat, and Codex Responses providers without an immediately available
credential. Each holds an `AuthResolver`; the first poll of each returned
stream resolves auth and may yield `ProviderError::CredentialNeeded` before
any HTTP request.

`AuthSource::EnvOAuthToken` stores `provider_id` separately from `env_var`.
Errors always name the provider, never `ANTHROPIC_OAUTH_TOKEN` or another
environment-variable name.

Factory-built mock HTTP tests perform two streams around a fake-store update.
The second stream must use the new credential, proving that construction did
not bake the original secret.

### Refresh timeout

OAuth refresh remains double-checked and holds the cross-process mutation lock
across re-read, HTTP refresh, and write. The entire refresh HTTP future is
wrapped in a bounded timeout shorter than the documented maximum lock hold.
Timeout drops the HTTP future, releases the RAII lock, writes no partial
credential, and returns a typed non-retryable failure.

### Mode behavior

Interactive mode retains one pending turn after a pre-output
`CredentialNeeded`. It presents explicit remediation; only a successful,
user-initiated `/login` retries that pending turn. The retry reuses the pending
turn and does not append a duplicate user message. Cancellation or login
failure leaves the turn failed and performs no retry.

JSON, RPC, and text non-interactive modes emit typed provider id and
`/login <provider>` remediation, then fail without starting a presenter,
opening a browser, or blocking for input.

For Anthropic, Copilot, and Codex, an auth-invalid response maps to
`CredentialRevoked`, is non-retryable, ends the current turn, performs no
automatic login, and causes no second HTTP request.

### Definition of done

- Missing credentials reach `CredentialNeeded` from inside each concrete
  stream rather than failing provider construction.
- The TUI same-turn retry is driven through production interaction code and
  produces exactly one user message and one post-login retry.
- JSON/RPC/non-interactive remediation is typed and non-blocking.
- Changed-store per-stream resolution and all-profile revocation are captured
  through factory-built providers and mock HTTP.
- Refresh timeout releases the lock and preserves the prior credential.

## 14.11 - Factory-Built Anthropic Cache Markers

Add a coding-agent-level integration test that constructs Anthropic through
the real provider factory, selects a built-in `ModelInfo` with its final nested
capabilities, injects auth, and captures the concrete stream request with mock
HTTP.

The capture verifies markers on the system prompt, last user text, last
assistant text, and last tool definition. It covers long retention
(`ttl: "1h"`), default/short ephemeral retention, explicit disablement, and
custom or unknown models whose capabilities default off.

Private request-body helpers may retain unit tests, but they cannot close this
task. The production factory, final capability lookup, provider stream, and
HTTP body must all participate in the owning scenario.

### Definition of done

- A factory-built capable model emits every required marker at the exact
  reviewed position and TTL.
- Disabled, custom, and unknown models emit no markers.
- The test fails if factory capability wiring or concrete stream assembly is
  removed, even if the private helper still passes.

## 14.12 - Usage and Cost Contract

### Public data model

`Usage` uses the reviewed fields:

```rust
pub cache_write_1h_tokens: Option<u64>,
pub reasoning_tokens: Option<u64>,
```

An absent upstream field remains `None`; an explicitly reported zero is
`Some(0)`. `CumulativeUsage` preserves the same distinction: its two child
totals and the corresponding `from_totals` inputs/accessors become optional.
A total remains `None` only when every contributing event omits that field;
once any event reports it, the total is `Some(sum)`.

Remove `CostBreakdown::cache_write_1h_cost`. The existing
`cache_write_cost` line contains the weighted short-cache remainder plus the
1-hour subset. Reasoning remains included in `output_cost`. `total_cost`
counts each parent bucket once.

This is an intentional public 0.x breaking correction and must be recorded in
`CHANGELOG.md` under Unreleased.

### Provider validation

Anthropic, OpenAI Chat, and OpenAI Responses parse optional children as
`u64`. If `cache_write_1h_tokens > cache_write_tokens` or
`reasoning_tokens > output_tokens`, the mapper emits a non-retryable
`ProviderError::StreamError` and no completion/Usage event containing the
invalid data. Absence, zero, and equality are valid.

### Persistence and resume

The session schema version does not change. Missing fields in older JSONL
entries deserialize as `None`; new optional fields serialize without storing
cost on messages. A production session-runtime test persists nonzero 1-hour
cache and reasoning subsets, reconstructs through a fresh reader/harness
resume path, and proves that cumulative usage and the cost summary are
identical before and after resume.

### Definition of done

- Public compile tests pin `Option<u64>` and the absence of the extra cost
  line.
- Valid provider fixtures preserve `None`, `Some(0)`, equality, and nonzero
  values.
- Invalid child subsets yield `StreamError` and no invalid Usage event.
- Cost tests prove weighted cache cost and no reasoning/cache double count.
- Nonzero breakdowns survive real session persistence and resume.

## 14.13 - Documentation, Verification, and Residual Closure

Task 14.13 owns final truth alignment after all runtime and public API work is
complete. It updates localized counterparts in the same change.

Required surfaces include:

- `docs/opi-spec.md` and `docs/opi-spec.zh.md`;
- root and crate `README.md`/`README.zh.md` files that describe Phase 14;
- exact `Request`, `Provider`, `Usage`, and `CostBreakdown` rustdoc snippets;
- `CHANGELOG.md` under `Unreleased`;
- CLI and TUI help/remediation text;
- the documented source layout, removing the unused auth directory claim.

The opi-ai README pair must state that capable Anthropic models do emit
`cache_control`; public product claims about keychain and OAuth are allowed
only after 14.8-14.10 pass.

### Runtime help evidence

- Drive `/help` through the production TUI dispatcher/command registry and
  assert that `/login` and `/logout` are discoverable.
- Drive the production JSON runner with a provider that yields
  `CredentialNeeded`; assert the NDJSON event shape, provider id,
  remediation, auth-failure exit code, and absence of prompting/blocking.
- Retain RPC and text non-interactive coverage showing that neither starts a
  login flow.

Source-text searches alone are guards, not runtime-help evidence.

### Acceptance-command repair

During reviewed reinit, replace nonexistent targets and zero-match filters in
all 29 Phase 14 acceptance scenarios with real targets and exact filters. Run
every command before graph confirmation and again in 14.13. Each must select
at least one intended test. The final scenario audit records target, filter,
selected count, exit code, and result; exit zero with zero selected tests is a
failure.

### Historical `api-map` disposition (superseded)

The following was the task-14.13 disposition. The 2026-07-17 pi comparison
proved its factual premise false: pi's GitHub Copilot provider already uses one
identity/catalog across three wire families. The alignment revision below
replaces this deferral with implementation tasks 14.15-14.18 and final
`implemented` evidence in 14.21.

Record `api-map` as `deferred-by-updated-design`. The prior trigger has fired,
but no in-tree provider currently needs one provider identity/catalog to route
models through two or more wire families. Explicit provider profiles remain
sufficient, so changing `Provider` and `ProviderCollection` now would add a
large boundary without a product driver.

New trigger: a concrete provider or config requirement must use one catalog
identity while selecting at least two concrete wire families, and explicit
profiles must be inadequate. When that occurs, a separate reviewed design must
define model-to-wire selection, per-stream auth, capability routing, and the
`ProviderCollection` boundary before implementation.

### Definition of done

- English and Chinese public documentation describe the final code exactly.
- Runtime TUI/JSON/RPC/text evidence covers help and remediation.
- All 29 ledger commands execute real tests.
- The historical `api-map` disposition is recorded; the later alignment
  revision owns its replacement.
- Full verification and phase exit pass.

## Audit Obligation Trace

The following 20 remediation obligations cover the accepted phase-exit
findings. Several audit bullets contained multiple closely related symptoms;
this table separates them where they require different fixes.

| ID | Obligation | Owner |
|---|---|---:|
| B01 | Install a production native keyring store | 14.8 |
| B02 | Stop corrupt/unknown envelopes from falling through to env | 14.8 |
| B03 | Allow stored-only model listing without an env secret | 14.8 |
| B04 | Test real async doctor/list orchestration rather than precomputed probes | 14.8 |
| B05 | Remove the unused auth-directory documentation claim | 14.13 |
| B06 | Make production login persist through the native store | 14.9 |
| B07 | Drive login/logout through the real TUI dispatcher and terminal guard | 14.9 |
| B08 | Let missing Anthropic credentials reach stream-time `CredentialNeeded` | 14.10 |
| B09 | Report provider id rather than env-var name | 14.10 |
| B10 | Bound refresh HTTP while holding the mutation lock | 14.10 |
| B11 | Add same-turn, JSON, changed-store, and all-profile revocation vertical evidence | 14.10 |
| B12 | Capture cache markers through a factory-built concrete stream | 14.11 |
| B13 | Correct the README claim that Anthropic markers are not emitted | 14.13 |
| B14 | Restore optional `u64` Usage child fields | 14.12 |
| B15 | Remove the unreviewed 1-hour CostBreakdown line | 14.12 |
| B16 | Reject malformed child subsets instead of clamping/dropping | 14.12 |
| B17 | Persist and resume nonzero new breakdowns end to end | 14.12 |
| B18 | Align normative signatures, Usage/cost docs, changelog, and product claims | 14.13 |
| B19 | Replace zero-test/nonexistent commands and add runtime TUI/JSON evidence | 14.13 |
| B20 | Renew the fired `api-map` residual disposition and trigger | 14.13 |

The six findings rejected by adversarial verification do not become tasks.
In particular, compatible Chat affinity remains opt-in, additive Usage serde
does not bump the session schema, SC4 remains met, and image generation is not
silently added to Phase 14.

## Verification Strategy

Each code task follows test-first execution: add a failing production-path or
contract test, implement the minimum fix, run the focused target, then run the
task tier gates. No test contacts a real keychain, OAuth endpoint, LLM API,
browser, user session directory, or interactive terminal.

After code changes, every task runs:

```text
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
```

Task 14.13 then runs, in order:

1. every focused corrective test;
2. all 29 acceptance-scenario commands, each selecting a test;
3. `cargo fmt --check --all`;
4. `cargo clippy --workspace --all-targets -- -D warnings`;
5. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`;
6. `cargo test --workspace --doc`;
7. `cargo test --workspace --all-targets`;
8. `scripts/opi-impl-smoke.ps1` from its actual reviewed location at execution
   time;
9. the Phase 14 artifact audit;
10. the five-lens Phase F evaluator and adversarial verification.

Any `not-met` criterion keeps Phase 14 active. The historical 14.13 allowance
for `deferred-by-updated-design` is superseded: no snapshot or archive commit
is created until the current alignment revision's exit gate passes and the
user accepts the separate archive gate.

## Commit Boundaries

The intended implementation commits are one per corrective task:

1. 14.8 native keyring and production probes;
2. 14.9 login/logout dispatcher and persistence;
3. 14.10 live auth and session interaction;
4. 14.11 factory-built Anthropic cache-marker evidence;
5. 14.12 Usage and cost contract correction;
6. 14.13 final documentation, verification metadata, and residual closure.

Every commit stages only exact task-owned paths. The baseline workflow-script
relocation is never swept into these commits.

## Superseded Phase F Closure Addendum (2026-07-17)

This addendum records the four findings that remain valid, but its
three-task graph and `api-map` deferral were superseded before implementation
by the user-approved pi-0.80.6 alignment revision below. Task 14.14 retains its
meaning. The old task 14.15 and 14.16 definitions are historical only and must
not be initialized or implemented.

Tasks 14.8-14.13 passed their task gates, but the independent Phase F
reconstruction still returned `not-met` for SC1, SC2, and SC3. The reports at
`target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_REPORT.md` and
`target/opi-artifacts/phase14-phase-exit/PHASE_EXIT_SCENARIO_AUDIT.md`
accepted four remaining findings:

| Finding | Criterion | Remaining gap |
|---|---|---|
| F14-01 | SC1 | The native-keyring host-selection test installs an injected mock directly and never traverses the production platform-selection layer. |
| F14-02 | SC2 | The normative source says every OAuth flow calls the manual-code presenter, while Copilot device-code intentionally does not. |
| F14-03 | SC2 | Dispatcher tests use a fake OAuth provider; concrete Anthropic, Copilot, and Codex tests bypass the dispatcher. |
| F14-04 | SC3 | The same-turn retry test composes helpers directly and never enters the outer interactive TUI path. |

SC4-SC8 remain met, all original Non-Goals remain respected, and `api-map`
remains exactly `deferred-by-updated-design`. The four findings add tasks
14.14-14.16 to this already registered corrective source; they do not reopen
or rewrite the passing task history of 14.8-14.13 and do not require a new
supplemental-source registry entry.

### Corrected OAuth flow semantics

Manual fallback is flow-specific:

- Anthropic and Codex use PKCE authorization-code flows. They attempt the
  loopback callback and may call `LoginPresenter::await_manual_code` for the
  reviewed manual code/URL-paste fallback.
- Copilot uses device-code authorization. It calls
  `LoginPresenter::present_device_code`, polls the device endpoint, and never
  calls `await_manual_code`. Displaying the verification URL and user code is
  the Copilot headless/SSH/manual path; there is no second code to paste back
  into opi.

The original provider/auth design is synchronized to this distinction in the
same design commit. This is a correction of the documented behavior, not a new
OAuth flow or a compatibility exception.

### Task graph

```text
14.14 native keyring host selection -----------+
                                                +-> Phase F rebuild
14.15 concrete OAuth dispatcher vertical path  |
  -> 14.16 outer TUI same-turn retry -----------+
```

Tasks 14.14 and 14.15 are logically independent. Task 14.16 follows 14.15 so
the outer TUI evidence consumes the corrected login semantics and dispatcher
service boundary. All three tasks set `evaluator_required = true`.

### 14.14 - Native Keyring Host Selection

Keep the production lifecycle unchanged: `install_native_keyring()` selects
the current target's native store through the same cfg-gated platform factory,
installs it as the `keyring-core` default, and returns the process-lifetime
guard that unsets the default store on drop.

Add only a package-private constructor seam at that platform-selection
boundary. The focused test injects a mock constructor but enters the same
host-selection layer used by `install_native_keyring()`; calling
`install_store(mock)` directly is not acceptance evidence. On the current
host, the test must prove:

- the expected cfg branch selected and invoked its constructor exactly once;
- the returned store became the `keyring-core` default;
- the installation guard retained and then released that default exactly
  once; and
- `BackendUnavailable` and other native-construction errors retain their
  existing typed mappings.

The test must not create, read, write, or delete a real OS-keychain entry. The
six-target release matrix remains the cross-target compile check; the focused
test executes the current host branch.

#### Definition of done

- The focused acceptance command selects at least one test that traverses the
  production platform-selection layer.
- Removing or disconnecting the cfg-selected factory makes that test fail.
- Default-store and guard cleanup behavior is proven without user-keychain IO.
- Native error classification is unchanged.

### 14.15 - Concrete OAuth Dispatcher Vertical Path

Correct the source semantics above and add a package-private endpoint-config
seam for constructing the built-in OAuth registry in tests. Production
construction continues to use the existing Anthropic, GitHub Copilot, and
OpenAI Codex endpoint constants; the seam is not a public configuration
surface.

Mock-backed integration tests must start `/login <provider>` and
`/logout <provider>` at `dispatch_auth_command`, use the real built-in registry
and concrete `AnthropicOAuthProvider`, `CopilotOAuthProvider`, and
`CodexOAuthProvider`, and finish through the injected locked credential store.
They must not call a concrete provider directly as a substitute for dispatcher
coverage.

The vertical tests verify, for each provider:

- the provider-specific authorization and token URL, required request headers,
  and persisted credential profile;
- locked mutation-coordinator persistence and deletion;
- terminal suspension/restoration exactly once on success and failure; and
- redacted typed failure when persistence or lock acquisition fails.

Anthropic and Codex exercise the PKCE manual-code seam without opening a real
browser or callback listener. Copilot asserts at least one
`present_device_code` call and exactly zero `await_manual_code` calls while the
mock device endpoint drives pending/success. No test contacts a real OAuth
endpoint, browser, terminal, or keychain.

#### Definition of done

- Dispatcher-to-real-provider login and logout are covered for all three
  built-ins.
- The tests fail if the dispatcher stops using the production registry, locked
  mutation coordinator, terminal guard, or provider-specific wire contract.
- Copilot's device-code presentation is proven as its headless/manual flow,
  with no paste-back request.
- The focused acceptance command selects the owning vertical tests.

### 14.16 - Outer TUI Credential Retry

Extract the smallest shared production state machine needed to process a
normal prompt result, retain a pre-output `CredentialNeeded` pending turn, and
handle the following auth command. Both the real `tui_event_loop` and a
debug-only scripted headless driver call this state machine. The driver is only
an input/terminal adapter; it must not duplicate prompt, pending-turn, login,
or retry branching.

The owning integration test enters `run_interactive_tui` and scripts:

```text
normal prompt -> CredentialNeeded(anthropic) -> /login anthropic -> exit
```

It asserts one persisted user message, two provider calls, and exactly one
retry of the original pending turn. A successful login for a different
provider, cancellation, presenter failure, login failure, or store failure
leaves the pending turn failed and performs zero retries. Those paths must not
append a duplicate user message.

The shared state machine is deliberately narrow. It does not redesign the TUI
event model, command registry, session schema, provider construction, or
non-interactive behavior, and the test uses fake terminal/store/provider
boundaries without user runtime state or network access.

#### Definition of done

- The success and negative tests enter the outer TUI function rather than
  manually composing retry helpers.
- The real event loop and scripted driver share the same prompt/auth/retry
  state transitions.
- Provider identity gates the retry and all failure/cancellation paths perform
  zero retries.
- The focused acceptance command selects the owning outer-path tests.

### Trace and final verification

The new ownership is:

| Criterion | New owner | Required closure |
|---|---:|---|
| SC1 | 14.14 | Current-host cfg selection, native-store installation, and guard lifecycle through one production selection seam. |
| SC2 | 14.15 | Truthful flow semantics plus dispatcher-to-real-provider login/logout coverage for all three built-ins. |
| SC3 | 14.16 | Outer-TUI `CredentialNeeded` login and same-turn retry with exact message/call counts and negative paths. |

The three affected acceptance commands must be replaced or extended so each
selects the intended new test. Final verification then reruns all 29 historical
commands, every corrective command, workspace format/clippy/doc/test/smoke
gates, the artifact audit, and the five-lens Phase F evaluator with independent
adversarial verification.

Archive remains a separate gate. It is permitted only when SC1-SC8 are all
`met`, every Non-Goal remains preserved, and the `api-map` residual retains its
exact `deferred-by-updated-design` citation.

### Task and commit boundaries

- 14.14 owns `native_keyring.rs`, its focused host-selection tests, and its
  acceptance metadata.
- 14.15 owns the synchronized OAuth design text, the internal built-in-registry
  endpoint seam, concrete dispatcher vertical tests, and its acceptance
  metadata.
- 14.16 owns the narrow interactive state-machine changes, outer-TUI tests,
  Phase F evidence, and its acceptance metadata.

Implementation uses one exact-path commit per task. The pre-existing
workflow-script relocation, `.gitignore`, skill registry, and ledger-schema
changes remain outside all three commits. There is no automatic archive commit.

## pi-0.80.6 Alignment Revision (2026-07-17)

This revision is the current implementation source for the remaining Phase 14
work. It incorporates the four Phase F findings above and a subsequent static
comparison against `.repo/pi-0.80.6`.

The comparison accepted three additional blocking specification gaps:

1. OpenAI Codex offers both Browser PKCE and Device Code login; the prior
   design modeled Browser PKCE only.
2. OpenAI Codex uses a dedicated `openai-codex-responses` wire; the prior
   design modeled it as configuration on the standard Responses provider.
3. GitHub Copilot is one provider identity/catalog spanning
   `anthropic-messages`, `openai-completions`, and `openai-responses`; the
   prior `api-map` deferral claimed no such in-tree product driver existed.

The user selected full provider/wire/catalog alignment while retaining opi's
reviewed security and correctness hardening. The approved choices are:

- keep the native keychain, typed auth errors, same-turn retry, strict Usage
  validation, reserved auth-header protection, and atomic catalog refresh;
- implement one generic typed model-to-wire routing substrate;
- route the built-in GitHub Copilot catalog through three wire families;
- add a dedicated OpenAI Codex Responses implementation and both Codex login
  methods;
- use pi provider ids `github-copilot` and `openai-codex`;
- provide no legacy provider-id alias and no keychain migration for the
  development-only `copilot`/`codex` entries;
- synchronize all runtime-affecting metadata in the reviewed pi-0.80.6
  GitHub Copilot and OpenAI Codex catalog snapshots; and
- expose the multi-wire model contract through both Rust and TOML.

### Intentional opi divergences

Full alignment here means provider identity, wire selection, OAuth flow
availability, model catalog, and request contract. It does not undo the
following explicit opi choices:

| Area | pi 0.80.6 | Binding opi behavior |
|---|---|---|
| Credential persistence | Plaintext `auth.json` | Native OS keychain; no opi-managed plaintext secret file |
| Auth errors | String/status guidance | Typed `CredentialNeeded` / `CredentialRevoked` |
| Missing credential interaction | Login changes future state | Successful explicit login may retry the same pre-output pending turn once |
| Usage children | Mappers may coerce absence and do not uniformly reject malformed subsets | Preserve `None` vs `Some(0)` and reject child-greater-than-parent |
| Extra headers | Request headers may override auth headers | Provider-managed auth headers remain reserved |
| Batch model refresh | Best-effort `allSettled` | Deterministic all-provider atomic replacement |
| Anthropic cache markers | Direct path uses pi's own placement | Preserve the already reviewed opi system + last user + last assistant + last tool placement |
| Model listing | May consult auth-backed state | Static audited catalog; no OAuth secret read or Copilot entitlement/model-enable call |

Per-call credentials, `onPayload`/`onResponse`, request-level retry fields,
end-to-end `SecretString` migration, new OAuth providers, and a session-schema
version redesign remain out of scope.

### Core wire and model architecture

The existing `opi_ai::ApiKind` classifies normalized assistant-message source.
It is not reused for exact provider routing. `opi-ai` adds a separate
non-exhaustive `WireApi` whose serialized names follow the reviewed API
identifiers. The complete initial set is:

- `anthropic-messages`;
- `openai-completions`;
- `openai-responses`;
- `openai-codex-responses`;
- `google-generative-ai`;
- `google-vertex`;
- `bedrock-converse-stream`; and
- `azure-openai-completions`.

OpenRouter, Mistral, and OpenAI-compatible profiles declare
`openai-completions`; direct Gemini declares `google-generative-ai`. Azure and
Vertex retain their transport-specific values even though they reuse lower
level Chat/Gemini encoders.

Every `ModelInfo` has one required `wire_api`. A public constructor and builder
surface lets external 0.x consumers construct the non-exhaustive value without
struct literals.

`ModelInfo` also becomes the source of truth for:

- the existing nested `ModelCapabilities`;
- a thinking-level map for `off`, `minimal`, `low`, `medium`, `high`, `xhigh`,
  and `max`, including an explicitly unsupported level;
- a tagged `WireCompat` value whose variant must match `wire_api`; and
- optional model pricing with deterministic input-token threshold tiers.

The new thinking levels are additive values in the existing thinking-level
session event. They do not change the session schema version, active-branch
selection, or context-reconstruction API. `ThinkingConfig` carries the selected
level in addition to the existing enabled/budget fields so each wire can apply
the model's map without reverse-engineering a level from a token budget.
Anthropic's existing budget behavior remains intact.

Model pricing chooses one effective `Pricing` before calling the existing
strict cost calculation; it adds no cost line and does not double-count cache
or reasoning subsets. Session cost resolution uses `ModelInfo` pricing first.
The current coding-agent pricing table remains a fallback only for models that
have not migrated embedded pricing; GitHub Copilot and OpenAI Codex catalog
entries must never use that fallback.

### `ApiMappedProvider`

`opi-ai` adds a public `ApiMappedProvider` with:

- one provider id;
- one model catalog;
- one checked `WireApi -> Box<dyn Provider>` route map; and
- construction-time validation that every catalog wire has exactly one route.

On `stream`, the mapped provider resolves the request's model in its own
catalog, selects the model's `wire_api`, and delegates to that route. Unknown
models, missing routes, and wire/compat mismatches produce typed,
non-retryable errors before network IO.

Route providers receive the mapped provider id and the same
`Arc<dyn AuthResolver>`. The resolver remains lazy: each returned stream
resolves auth immediately before its HTTP request. `ResolvedAuth` expands to:

```rust
pub struct ResolvedAuth {
    pub scheme: AuthScheme,
    pub secret: SecretString,
    pub base_url: Option<String>,
    pub account_id: Option<String>,
}
```

Static API-key resolvers return `None` for the two new fields. GitHub Copilot
uses `base_url` for per-stream enterprise routing. OpenAI Codex requires
`account_id`. No route resolves auth a second time at the mapped-provider
layer.

Static mapped providers return `Ok(None)` from `refresh_models`. A future
dynamic mapped provider must return one complete catalog snapshot; the
collection continues to install all provider snapshots atomically. This
revision adds no production refresh trigger.

### TOML custom-provider contract

The new user surface is `[providers.custom.<id>]`:

```toml
[providers.custom.acme]
name = "Acme"
base_url = "https://api.acme.example"
api_key_env = "ACME_API_KEY"
auth_scheme = "bearer"
api = "openai-completions"

[[providers.custom.acme.models]]
id = "claude-model"
display_name = "Claude Model"
api = "anthropic-messages"
base_url = "https://api.acme.example"
context_window = 200000
max_output_tokens = 32000
```

Provider-level fields own the shared credential source, auth scheme, default
base URL, proxy, headers, and optional default API. A model may override its
API, base URL, capabilities, thinking map, wire-specific compatibility
metadata, and pricing. If neither provider nor model supplies an API, config
loading fails.

The TOML surface accepts `anthropic-messages`, `openai-completions`, and
`openai-responses`. `openai-codex-responses` is subscription-specific and
cannot be selected by a custom provider. The existing
`[providers.openai_compatible]` table remains the single-wire OpenAI
Completions shorthand, but lowers into the same mapped-provider construction
path rather than retaining a second dispatcher.

Configuration rejects, before provider construction or network IO:

- unknown or disabled wire names;
- duplicate model ids;
- a catalog wire without a route;
- compatibility fields for the wrong wire;
- non-positive token limits or invalid/overlapping price tiers;
- an absent provider/model API;
- invalid header names or values; and
- provider-managed auth-header overrides.

One provider shares one credential source and auth scheme across its routes.
Per-wire credentials and auth schemes are not part of this design.

### GitHub Copilot parity

The built-in id and keychain account key are `github-copilot`. The provider
owns one checked-in pi-0.80.6 catalog snapshot and three routes:

| `WireApi` | Endpoint |
|---|---|
| `AnthropicMessages` | `/v1/messages` |
| `OpenAiCompletions` | `/chat/completions` |
| `OpenAiResponses` | `/responses` |

All routes use Bearer auth, including Anthropic Messages. Each receives the
same lazy `AuthSource::Store`. A store update to either the access token or
enterprise base URL affects the next stream without reconstructing the
provider.

The wire contract includes the reviewed static `User-Agent`,
`Editor-Version`, `Editor-Plugin-Version`, and `Copilot-Integration-Id`
headers. It derives `X-Initiator: user|agent` from the last message, emits
`Openai-Intent: conversation-edits`, and emits
`Copilot-Vision-Request: true` when user or tool-result content contains an
image.

Every catalog entry carries the reviewed id, display name, wire, capability
limits, thinking map, compatibility metadata, and pricing. A checked-in
fixture records pi version, source path, and SHA-256. Acceptance compares the
entire Rust catalog to that offline fixture. CI never depends on `.repo/`.

The static snapshot is not filtered through Copilot's live account-entitlement
or model-enable endpoints. This preserves secret-free model listing and is an
explicit opi divergence, not a catalog-parity claim.

### OpenAI Codex parity

The built-in id and keychain account key are `openai-codex`. No `codex` alias
or credential migration is provided. Existing development credentials require
an explicit new login.

`LoginPresenter` adds a typed login-method selector:

```rust
pub enum OAuthLoginMethod {
    Browser,
    DeviceCode,
}
```

`/login openai-codex` always presents Browser as the default and Device Code as
the headless option. Cancellation returns a typed non-retryable cancellation
and writes nothing.

Browser login retains the PKCE S256 flow, loopback callback, manual
code/redirect-URL paste, state validation, timeout, and cancellation race.

Device Code login:

1. posts the client id to `/api/accounts/deviceauth/usercode`;
2. presents the public user code and `/codex/device` verification URI;
3. polls `/api/accounts/deviceauth/token`, honoring pending and slow-down;
4. receives an authorization code and code verifier; and
5. exchanges them at `/oauth/token` using the device callback redirect URI.

Denial, expiry, timeout, cancellation, invalid responses, and refresh failures
are typed and redacted. The refresh HTTP future remains bounded while the
cross-process mutation lock is held.

`OAuthCredential`, the persisted credential envelope, and `ResolvedAuth` add
optional non-secret `account_id`. Codex login and refresh require the
`chatgpt_account_id` field under the `https://api.openai.com/auth` JWT claim
and fail if it is absent. Anthropic and GitHub Copilot store `None`.

`opi-ai` adds `openai_codex_responses.rs`. This provider owns:

- `WireApi::OpenAiCodexResponses`;
- default base URL `https://chatgpt.com/backend-api`;
- endpoint `/codex/responses`;
- the Codex request body;
- its SSE event mapping and provider-error classification; and
- `Authorization`, `chatgpt-account-id`, `originator: opi`,
  `OpenAI-Beta: responses=experimental`, `accept: text/event-stream`,
  `session-id`, and `x-client-request-id`.

It may reuse extracted low-level standard-Responses parsing helpers. It must
not be constructed by toggling Codex flags on `OpenAiResponsesProvider`.

The full reviewed pi-0.80.6 Codex catalog is checked against the same kind of
offline provenance fixture as GitHub Copilot.

### Dispatcher, modes, and outer TUI

The production OAuth registry contains exactly `anthropic`,
`github-copilot`, and `openai-codex`.

`dispatch_auth_command` receives endpoint configuration, HTTP client, locked
credential store, presenter, and terminal guard as injectable services. Tests
replace only those boundaries. They start at `/login <provider>` or
`/logout <provider>`, use the real registry and concrete OAuth provider, and
finish at locked persistence/deletion.

Terminal suspension and restoration remain RAII-controlled and occur exactly
once on success, failure, selection cancellation, presenter failure, OAuth
timeout, and store/lock failure.

The outer interactive state machine remains:

```text
normal prompt
  -> pre-output CredentialNeeded(provider)
  -> retain one pending turn
  -> explicit login for the same provider succeeds
  -> retry the original turn once without a duplicate user message
```

A successful login for another provider, selection cancellation, presenter
failure, OAuth failure, store failure, or terminal restoration failure performs
no retry. Mid-stream auth rejection is `CredentialRevoked`, ends the turn, and
never starts login.

JSON, RPC, and text modes emit the canonical provider id and
`/login <provider>` remediation, then fail without constructing a presenter,
opening a browser, or waiting for input.

### Typed failures

The implementation must distinguish:

- unknown model;
- missing model-to-wire route;
- model wire and compatibility mismatch;
- unknown/disabled config wire;
- Codex token missing account id;
- login-method cancellation or invalid selection;
- `CredentialNeeded`;
- `CredentialRevoked`; and
- existing timeout/network/rate-limit/provider classes.

Configuration and route-shape errors are non-retryable and occur before network
IO. A 401/403 from Anthropic, any GitHub Copilot route, or OpenAI Codex remains
typed `CredentialRevoked`.

### Replacement task graph

The shipped history through 14.13 remains immutable. Task 14.14 retains the
native-keyring host-selection definition above. The historical 14.15 and 14.16
definitions are replaced with this graph:

```text
14.14 native keyring host selection ------------------------------+
                                                                   |
14.15 WireApi, ModelInfo metadata, and provider-id migration       |
  -> 14.16 ApiMappedProvider and TOML custom providers             |
       -> 14.17 GitHub Copilot three-wire catalog -----------+     |
       -> 14.18 OpenAI Codex wire/catalog/dual login --------+     |
                                                            |     |
                                                            v     |
                                  14.19 concrete OAuth dispatcher  |
                                    -> 14.20 outer TUI retry -------+
                                                                   |
                                                                   v
                                                        14.21 final alignment
```

All tasks set `evaluator_required = true`.

### 14.14 - Native Keyring Host Selection

The retained definition above is binding. Its acceptance test must traverse
the same cfg-gated platform-selection function as
`install_native_keyring()`, prove constructor/default-store/guard lifecycle,
and perform no real user-keychain operation.

### 14.15 - Wire and Model Foundation

This task:

- adds `WireApi` without changing `ApiKind` message semantics;
- adds required `ModelInfo::wire_api`, thinking maps, wire-specific
  compatibility metadata, model pricing, and deterministic pricing tiers;
- carries the selected thinking level through `ThinkingConfig` and makes
  `ModelInfo` pricing authoritative before the legacy pricing fallback;
- migrates every workspace `ModelInfo` construction through public
  constructors/builders;
- adds the additive thinking-level values without changing the session schema
  version;
- changes provider ids, OAuth registry ids, diagnostics, model specs, and
  keychain keys from `copilot`/`codex` to
  `github-copilot`/`openai-codex`; and
- adds no legacy alias or credential migration.

Definition of done:

- external-consumer compile tests use public constructors;
- every built-in model has one exact wire;
- pricing-tier boundary tests cover equality and the first greater threshold;
- unsupported thinking levels are rejected before request construction;
- old provider ids are rejected with canonical remediation; and
- all existing single-wire providers still dispatch through their declared
  wire.

### 14.16 - API Map and TOML

This task adds `ApiMappedProvider`, the `[providers.custom]` schema, and the
single construction path used by both custom mapped providers and existing
OpenAI-compatible profiles.

Definition of done:

- one provider/catalog dispatches representative models through Anthropic,
  OpenAI Completions, and OpenAI Responses;
- each route shares one resolver and uses per-stream auth;
- unknown model, missing route, and mismatched compatibility errors occur
  before HTTP;
- provider API defaults and model overrides obey the reviewed precedence;
- model base URL overrides beat the shared provider default;
- invalid wire, tier, header, and auth combinations fail at config load; and
- list-models/picker output contains one provider identity rather than hidden
  route providers.

### 14.17 - GitHub Copilot Multi-Wire Catalog

This task constructs the built-in `github-copilot` mapped provider from the
reviewed pi-0.80.6 fixture and implements the three exact wire contracts and
dynamic headers.

Definition of done:

- the full catalog equals the checked-in fixture;
- one representative model per wire reaches its exact endpoint;
- Anthropic Copilot uses Bearer rather than `x-api-key`;
- static and dynamic Copilot headers match the reviewed contract;
- image input toggles `Copilot-Vision-Request`;
- a changed token or enterprise base URL affects the next stream; and
- auth rejection on every route is non-retryable `CredentialRevoked`.

### 14.18 - OpenAI Codex Wire, Catalog, and Login Methods

This task adds the dedicated provider, the full catalog snapshot, the Codex
login selector, Device Code flow, account-id persistence, and the canonical
`openai-codex` identity.

Definition of done:

- Browser is the default selection and preserves PKCE/manual behavior;
- Device Code covers pending, slow-down, success, denial, expiry, timeout, and
  cancellation without calling `await_manual_code`;
- login and refresh reject tokens without account id;
- the concrete provider uses the dedicated module and exact base/path/headers;
- no Codex construction path uses `OpenAiResponsesProvider` compatibility
  flags;
- the full catalog equals the checked-in fixture; and
- no credential, authorization code, device secret, JWT, or envelope appears
  in captured output/errors.

### 14.19 - Concrete OAuth Dispatcher Vertical Path

This task replaces the historical 14.15 definition. Tests start at
`dispatch_auth_command`, use the real built-in registry and concrete
Anthropic, GitHub Copilot, and OpenAI Codex OAuth providers, and finish through
the locked credential store.

Definition of done:

- login/logout works through the production dispatcher for all canonical ids;
- both Codex login selections traverse the dispatcher;
- exact provider URLs, request fields, headers, and persisted credential
  profiles are captured;
- success/failure/cancellation restores terminal state exactly once;
- Copilot and Codex Device Code call `present_device_code` and never
  `await_manual_code`; and
- store/lock failures are typed, redacted, and never report success.

### 14.20 - Outer TUI Credential Retry

This task replaces the historical 14.16 definition. The real
`tui_event_loop` and a debug-only scripted headless adapter share the same
production prompt/auth/pending-turn state machine.

Definition of done:

- tests enter `run_interactive_tui`;
- a normal prompt followed by pre-output `CredentialNeeded(anthropic)` and
  successful `/login anthropic` produces one user message, two provider calls,
  and one retry;
- a different provider, selection cancellation, presenter failure, OAuth
  failure, store failure, or terminal failure produces zero retries;
- negative paths append no duplicate user message; and
- JSON/RPC/text modes remain non-blocking and never construct a presenter.

### 14.21 - Documentation, Acceptance, and Phase Exit

This task updates all English/Chinese public documentation, rustdoc, CLI/TUI
help, and `CHANGELOG.md`. It removes every claim that:

- Copilot is a Chat-only compatibility profile;
- Codex is standard Responses with flags;
- Codex supports Browser PKCE only;
- provider ids are `copilot` or `codex`; or
- `api-map` remains deferred.

`api-map` is recorded as `implemented` with exact task, fixture, and
acceptance-test citations. No other Phase 14 residual may use
`deferred-by-updated-design` unless a currently binding source names it.

The final acceptance run executes:

1. every focused test for 14.14-14.20;
2. all 29 historical Phase 14 acceptance commands;
3. every new alignment acceptance command;
4. `cargo fmt --check --all`;
5. `cargo clippy --workspace --all-targets -- -D warnings`;
6. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`;
7. `cargo test --workspace --doc`;
8. `cargo test --workspace --all-targets`;
9. `scripts/opi-impl-smoke.ps1` from its reviewed location at execution time;
10. the Phase 14 artifact audit; and
11. the five-lens Phase F evaluator with independent adversarial verification.

Every acceptance command must select at least one intended test. The scenario
audit records target, filter, selected count, exit code, and result. Exit zero
with zero selected tests is a failure.

The checked-in pi fixtures carry source version/path and SHA-256. Tests remain
offline and never contact a real keychain, browser, terminal, OAuth endpoint,
provider endpoint, user config directory, or session directory.

### Updated obligation trace

| Obligation | Owner |
|---|---:|
| F14-01 production native-keyring host selection | 14.14 |
| Exact wire identity and complete model metadata | 14.15 |
| Canonical provider ids with explicit no-migration behavior | 14.15 |
| Public Rust api-map and TOML multi-wire provider contract | 14.16 |
| GitHub Copilot three-wire catalog and request behavior | 14.17 |
| Codex dedicated wire and pi-0.80.6 catalog | 14.18 |
| Codex Browser/Device Code selection and account-id persistence | 14.18 |
| F14-02 truthful flow-specific manual semantics | 14.18 / 14.19 |
| F14-03 dispatcher-to-real-provider coverage | 14.19 |
| F14-04 outer-TUI same-turn retry | 14.20 |
| Runtime help, fixture provenance, acceptance repair, `api-map` implementation evidence, and Phase F rebuild | 14.21 |

### Commit boundaries

Implementation uses one exact-path commit per task:

1. 14.14 native-keyring host selection;
2. 14.15 wire/model foundation and provider-id migration;
3. 14.16 api-map and TOML custom providers;
4. 14.17 GitHub Copilot multi-wire catalog;
5. 14.18 OpenAI Codex wire/catalog/login methods;
6. 14.19 concrete OAuth dispatcher vertical path;
7. 14.20 outer TUI retry; and
8. 14.21 final documentation and Phase F evidence.

The pre-existing workflow-script relocation, `.gitignore`, skill registry, and
ledger-schema changes remain outside these commits unless separately assigned.
There is no automatic archive commit.

### Exit gate

Archive remains separate. It is permitted only when:

- SC1-SC8 are all `met`;
- `api-map` is `implemented`, not deferred;
- every obligation in this revision is met by its production-path evidence;
- all original Non-Goals and intentional opi divergences are preserved; and
- the user accepts the archive gate.

## External Technical References

- `keyring-core` requires applications to install a default credential store:
  <https://docs.rs/crate/keyring-core/1.0.0>
- The current keyring v1 compatibility implementation selects Apple Keychain,
  Windows Credential Manager, and zbus Secret Service stores:
  <https://docs.rs/keyring/latest/src/keyring/v1.rs.html>
- Windows native store:
  <https://docs.rs/windows-native-keyring-store/latest/windows_native_keyring_store/>
- Apple native store:
  <https://docs.rs/crate/apple-native-keyring-store/latest>
- zbus Secret Service store:
  <https://docs.rs/zbus-secret-service-keyring-store/latest/zbus_secret_service_keyring_store/>
