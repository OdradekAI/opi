# Phase 14 Exit Remediation Design

Status: user-approved on 2026-07-14; Phase F closure addendum user-approved
on 2026-07-17.

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
6. Reassess `api-map` now that its old trigger fired, but do not expand Phase
   14 with `api-map` implementation.

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

The remediation also does not implement `api-map`, add a production model
refresh trigger, or reopen SC4/SC7 without new evidence of a regression.

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

### `api-map` disposition

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
- `api-map` has the exact updated-design citation and new trigger above.
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

Any `not-met` criterion keeps Phase 14 active. No snapshot or archive commit is
created until the phase-exit trace contains only `met` or an exactly cited
`deferred-by-updated-design`, every Non-Goal remains respected, and the user
accepts the separate archive gate.

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

## Phase F Closure Addendum (2026-07-17)

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
