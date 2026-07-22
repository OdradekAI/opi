# Phase 14 Provider & Auth Design

Historical note: under the 2026-07-10 roadmap redesign
(`docs/superpowers/plans/2026-07-10-phase-roadmap-redesign-map.md`), Phase 14 is
**Provider & Auth**. The prior Phase 14 (TUI product polish,
`2026-06-24-phase14-tui-product-polish-design.md`) is renumbered to Phase 17.
This doc synthesizes tickets T1 (credential store), T2 (OAuth + per-request
auth), and T3 (opi-ai Request enrichment), all resolved 2026-07-11.

> Remediation status (updated 2026-07-17): tasks 14.1-14.13 shipped, but the
> latest Phase F reconstruction still leaves SC1-SC3 `not-met`. A subsequent
> comparison with pi 0.80.6 also found that this historical design incorrectly
> narrowed Codex to browser PKCE, modeled Codex as a Responses compatibility
> profile, and deferred `api-map` even though GitHub Copilot is a concrete
> multi-wire provider. The reviewed corrective source is
> `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`; its
> 2026-07-17 alignment revision adds tasks 14.14-14.21 and supersedes this
> file wherever provider ids, login methods, wire identity, model metadata,
> catalog scope, or `api-map` differ.

## Overview

Phase 14 closes the provider/auth cluster (cluster A) identified by the
pi-0.80.6 realignment under posture B (strategic gap-closing). It introduces an
OS-keychain credential store, OAuth login and token refresh for the three
providers pi supports (Anthropic, GitHub Copilot, OpenAI Codex), per-request
auth re-resolution on the live run path, and additive enrichment of the
`opi_ai::Request` / `Usage` / `CostBreakdown` / `ModelInfo` surfaces. All three
subsystems are Rust-native and preserve the construction-ownership invariant.

The phase is implementation-ready: each subsystem lists concrete types,
signatures, crate placement, and verified source touch points. `opi-implement`
breaks this doc into tasks; it is not itself a task list.

## Goals

- An OS-keychain credential store with env-var fallback, atomic cross-process
  locking, and redacted probing — no opi-managed plaintext credential file.
- OAuth (PKCE authorization-code and device-code flows) for Anthropic, GitHub
  Copilot, and OpenAI Codex, with double-checked-locking token refresh,
  flow-specific manual fallback, and an explicit Browser/Device Code choice
  for OpenAI Codex.
- Per-request auth re-resolution on the live run path without moving dispatch
  onto `ProviderCollection` (preserving the Phase 10 boundary).
- Additive `Request` scalars (`timeout`, `extra_headers`, `cache_retention`,
  `session_id`), `Usage` cache-and-reasoning breakdown fields with corrected
  cost calculation, migration of the existing cross-provider
  `ModelCapabilities` type onto `ModelInfo`, and a dynamic `refresh_models`
  trait method.
- Anthropic prompt-cache markers gated by model capability, not wire-compat
  quirk.
- `opi doctor` and `--list-models` reflect stored credentials without reading
  the secret.

## Non-Goals

- No opi-managed plaintext credential file (rejected in T1 D1; env-var fallback
  sidesteps the encryption-key bootstrap problem on headless hosts).
- No auto-relogin mid-stream (T2 D5): a revoked token stops the turn and prompts
  re-login; the running turn is not silently re-authorized.
- No per-call credential (`apiKey` / `env`) or provider-managed auth-header
  override (pi `ApiStreamOptions`). `Request::extra_headers` remains additive
  for non-reserved transport headers; reserved auth headers are rejected.
- No `onPayload` / `onResponse` streaming hooks (T3 3d) — deferred to fog;
  distinct from Phase 17 T14's turn-level provider hook.
- No `maxRetries` / `maxRetryDelay` on `Request` — retry policy is `opi-agent`'s
  `agent_loop`, not opi-ai wire.
- No end-to-end `SecretString`-through-provider-construction refactor (T1 D5
  scope cap; deferred follow-up).
- No new OAuth providers beyond the three pi ships.
- No session-schema or context-reconstruction changes. TUI changes are limited
  to the reviewed `/login`, `/logout`, `CredentialNeeded` presenter, and
  raw/alternate-screen suspension around login; unrelated TUI product changes
  remain in Phases 13 and 17.

## Relationship to pi

pi persists credentials in a plaintext `auth.json`. opi diverges: the OS
keychain is the primary store for all persisted credentials (API keys and OAuth
tokens), with env-var fallback for API keys on hosts without a keychain daemon.
This is a deliberate security improvement, not parity.

pi has three OAuth providers: Anthropic, GitHub Copilot, and OpenAI Codex.
Anthropic uses PKCE authorization-code with a local `127.0.0.1` callback and
manual code/URL-paste fallback. Copilot uses device-code and never calls
`LoginPresenter::await_manual_code`. Codex first asks the user to choose
Browser (default) or Device Code (headless): Browser uses PKCE callback/manual
paste, while Device Code presents the verification URL and user code and
polls without paste-back.

pi also gives every model an exact wire identity. GitHub Copilot owns one
provider id and catalog spanning `anthropic-messages`, `openai-completions`,
and `openai-responses`; OpenAI Codex uses the dedicated
`openai-codex-responses` wire. The corrective source adopts that architecture
while retaining opi's native-keychain, typed-error, strict-usage,
reserved-header, same-turn-retry, and atomic-refresh hardening.

pi's live run path routes through `ProviderCollection` per request for auth
re-resolution. opi keeps the live path on `Box<dyn Provider>`
(`agent_loop.rs:118` calls `context.provider.stream(request)` directly) and
instead makes the three approved live provider paths hold an injected
`AuthResolver` that re-resolves on each `stream()`. `ProviderCollection` stays off the live path
(preserves the Phase 10 boundary); refresh and locking are centralized in the
credential store.

## Load-bearing invariant

`opi-agent` must not construct providers or own provider/auth configuration.
The abstract types live in `opi-ai`; the construction, IO, env, keychain, and
HTTP-refresh implementations live in `opi-coding-agent`:

- `AuthDescriptor` (`provider_collection.rs:99`, `#[non_exhaustive]` at `:98`)
  and `ProviderCollection` (`provider_collection.rs:245`) are already defined
  in `opi-ai`.
- `provider_factory.rs` (`crates/opi-coding-agent/src/provider_factory.rs`)
  imports them from `opi_ai` (`:44`) and constructs provider instances and the
  listing collection (`build_provider` at `:620`, `build_collection_for_listing`
  at `:961`).
- `agent_loop.rs:118` calls `context.provider.stream(request)`; opi-agent never
  constructs a provider.

Phase 14 honors this split. The new `CredentialStore`, `Credential`,
`CredentialSource`, `OAuthProvider`, `OAuthCredential`, `AuthResolver`,
`ResolvedAuth`, and `LoginPresenter` traits/types are defined in `opi-ai`
(abstract, no IO/env). Traits used behind `dyn` are object-safe and return a
boxed future; native `async fn` traits are reserved for monomorphized internal
code. Their concrete implementations — keychain/env/resolver stores, the three
OAuth providers and their registry, the `AuthSource` resolver, and presenter
impls — live in `opi-coding-agent`.

`opi-agent` remains unchanged for provider construction and auth resolution:
auth is resolved inside the three approved Anthropic, Copilot, and Codex
provider streams, reached through the existing `Box<dyn Provider>` seam. T3 makes one narrow, auth-independent
change to `opi-agent`: it carries the active session-affinity id through
`Agent` / `AgentLoopContext` into `Request::session_id`.

`ModelCapabilities` is not a new type. It already exists in
`opi_ai::registry` and currently mirrors flattened `ModelInfo` fields. T3
migrates that existing type onto `ModelInfo`, extends it with exact cache
capabilities, and routes registry/collection capability queries through the
single nested value. It does not create a second same-named type.

## Implementation Priority and Crate Boundaries

| Priority | Scope | Owner | Requirement |
|---|---|---|---|
| P0 | `CredentialStore` trait + `Credential` type | `opi-ai` | Abstract `read`/`write`/`delete`/`probe`; no IO or env access. |
| P0 | `CredentialSource` three-state enum | `opi-ai` | `Present(label)` / `Absent` / `BackendUnavailable(reason)` so doctor distinguishes "missing entry" from "no keychain daemon". |
| P0 | `AuthDescriptor::StoreCredential` variant | `opi-ai` | Additive `{ key, display_source }`; cheap, `Clone`, no secret. New match arms in `doctor`, `dispatch_stream`, `--list-models`. |
| P0 | `KeychainCredentialStore` + `EnvCredentialSource` + `CredentialResolver` + `fs4` global lock | `opi-coding-agent` | `keyring-core` primary; env fallback; single `<user_config_dir>/opi/credential.lock`; acquire-then-re-read. |
| P0 | `OAuthProvider` trait + `OAuthCredential` | `opi-ai` | `id()` / boxed-future `login(presenter)` / boxed-future `refresh(refresh)`; flow-agnostic and object-safe for the heterogeneous registry. |
| P0 | Three OAuth impls + `OAuthProviderRegistry` | `opi-coding-agent` | Anthropic (PKCE + `127.0.0.1` callback), Copilot (device-code), Codex (Browser PKCE + Device Code selector); register `anthropic`, `github-copilot`, and `openai-codex`. |
| P0 | object-safe auth-resolution seam | `opi-ai` / `opi-coding-agent` | `AuthResolver` + `ResolvedAuth` are abstract in `opi-ai`; the concrete `AuthSource` (`Baked` / `Store` / `EnvOAuthToken`) lives in `opi-coding-agent`, implements the boxed-future seam, and is resolved per `stream()`. |
| P0 | `Request` enrichment scalars | `opi-ai` | `timeout: Option<Duration>`, `extra_headers: HeaderMap`, `cache_retention: Option<CacheRetention>`, `session_id: Option<String>` (additive, default None/empty). |
| P0 | session-affinity propagation | `opi-agent` / `opi-coding-agent` | `CodingHarness` sets the active `SessionCoordinator` id on `Agent`; `AgentLoopContext` copies it into every `Request`, including after resume/fork. Provider-specific wire mappings are cache-gated and compatibility-gated. |
| P0 | `Usage` cache + reasoning breakdowns and cost calculation | `opi-ai` | Optional `cache_write_1h_tokens` is a subset of cache-write tokens; optional `reasoning_tokens` is a subset of output tokens. `CostBreakdown` remains `Copy`; cost stays computed separately and never double-counts either subset. |
| P0 | Existing `registry::ModelCapabilities` migrated onto `ModelInfo` | `opi-ai` | Make the existing type `#[non_exhaustive]`, add exact `supports_cache_control` and `supports_long_cache_retention` fields, embed it in `ModelInfo`, and migrate registry/collection capability queries. Custom/unknown defaults stay off. |
| P0 | Anthropic `cache_control` markers | `opi-ai` (anthropic provider) | Emit `{type:ephemeral,ttl}` on system + last user/assistant text + last tool def when `supports_cache_control`; `ttl='1h'` when `supports_long_cache_retention && cache_retention==Long`. |
| P1 | `refresh_models` substrate on `Provider` trait | `opi-ai` | Object-safe boxed-future `Result<Option<Vec<ModelInfo>>, ProviderError>`, default `Ok(None)`; mutable `ProviderCollection::refresh` atomically replaces successful dynamic catalogs only when the full refresh batch succeeds. Phase 14 adds no production trigger. |
| P1 | `LoginPresenter` trait + impl | `opi-ai` (trait) / `opi-coding-agent` (impl) | `TuiLoginPresenter`; PKCE flows support manual code paste, device flows present and poll without paste-back, and Codex selects Browser or Device Code. RPC/JSON/text remediation never constructs a presenter. |
| P1 | `doctor` `CredentialProbe` store arm | `opi-coding-agent` | Extend the `EnvApiKey` / `StaticApiKey` arms at `doctor.rs:619-625` to report `store.probe` presence without reading the secret. |
| P1 | Phase documentation, localized mirrors, changelog, and final guards | `workspace docs/tests` | After 14.1-14.6, update public docs/help and localized counterparts, record public 0.x breaks/additions, verify every Non-Goal, and align the Phase 14 status text without claiming runtime model refresh. |

Phase 14 must not satisfy product acceptance with abstract traits alone.
Product-scoped P0 items need a traced production path into the concrete
provider or command behavior, exercised by mock-backed integration tests
(never a real LLM API). The public `Request` knobs that have no Phase 14
config/harness producer and the 3e model-refresh API are explicitly substrate;
their tests must say so instead of inventing a production caller.

## Design

### T1 — Credential store model

**Backend.** The OS keychain (`keyring-core`) is primary for all persisted
credentials — API keys and OAuth tokens. There is no opi-managed plaintext
credential file. On hosts without a keychain daemon (headless, SSH, CI), API
keys fall back to the existing env-var path with a clear diagnostic; OAuth login
requires a keychain because refresh tokens must persist. Headless users use env
API keys.

**Contract (opi-ai).** A `CredentialStore` trait and a `Credential` type, both
abstract — no IO and no env access:

```rust
pub enum Credential {
    ApiKey(SecretString),
    OAuthToken {
        access: SecretString,
        refresh: SecretString,
        expires_at: Option<OffsetDateTime>,
        base_url: Option<String>,
        account_id: Option<String>,
    },
}

pub type BoxAuthFuture<'a, T> =
    Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait CredentialStore: Send + Sync {
    fn read<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<Option<Credential>, CredentialStoreError>>;
    fn write<'a>(
        &'a self,
        provider_id: &'a str,
        cred: &'a Credential,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>>;
    fn delete<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, Result<(), CredentialStoreError>>;
    fn probe<'a>(
        &'a self,
        provider_id: &'a str,
    ) -> BoxAuthFuture<'a, CredentialSource>;
}
```

The boxed-future methods are object-safe for `Arc<dyn CredentialStore>` and
leave refresh-on-read asynchronous. They also preserve backend errors instead
of collapsing an unavailable keychain into the same value as a missing entry.
Native `async fn` traits remain available to monomorphized internal helpers,
but not to heterogeneous registries or provider-held trait objects.

**Impls (opi-coding-agent).** `KeychainCredentialStore` (via `keyring-core`),
`EnvCredentialSource`, and a composing `CredentialResolver` own all IO and env
access. This is where the credential file lock lives, not in the trait.

Keychain entries use service `opi`, provider id as the account key, and a
versioned JSON envelope whose v1 payload distinguishes API-key and OAuth
credentials and preserves OAuth `base_url` plus the optional non-secret Codex
`account_id`. Missing entries return `Ok(None)`;
malformed JSON, an unknown envelope version/type, and keychain backend failure
return distinct `CredentialStoreError` values and are never collapsed into
absence or env fallback. Tests inject the keyring backend and never access the
user keychain.

**AuthDescriptor (opi-ai).** Additive variant, no secret:

```rust
AuthDescriptor::StoreCredential { key: String, display_source: String }
```

`AuthDescriptor` is `#[non_exhaustive]` (`provider_collection.rs:98`), so the
variant is additive. The existing wildcard in `doctor` must not silently absorb
it: Phase 14 adds an explicit `AuthDescriptor::StoreCredential` arm that calls
`store.probe` and distinguishes `Present`, `Absent`, and
`BackendUnavailable` without reading the secret. The asynchronous outer doctor
and model-listing command paths await `store.probe`, then pass only its redacted
state into their existing synchronous report/formatting helpers.
`--list-models` exposes only the display label and redacted probe state.
`dispatch_stream` retains only its non-live status-gate role: construction must
inject a precomputed redacted probe state because the secret-free descriptor
cannot perform asynchronous IO. It must not become the live credential
resolver.

**Locking.** A single global `fs4` advisory lock at
`<user_config_dir>/opi/credential.lock` (the lock file holds no secret — it is
pure coordination, because the OS keychain has no cross-"read → refresh → write"
transaction). The exclusive lock wraps every store mutation (`write`, `delete`,
and OAuth refresh in T2), applied uniformly with acquire-then-re-read (pi's
`auth-storage.ts:505-516` pattern: concurrent opi instances share one op), plus
a short timeout and diagnostic on contention rather than an infinite block.
The coding-agent lock coordinator also exposes package-private unlocked backend
operations so OAuth refresh can hold one lock across read, HTTP refresh, and
write without recursively acquiring the public `write` lock. Login, logout,
direct writes, and refresh all use the same coordinator; no mutation bypasses
it.

**In-memory and redaction.** `secrecy::SecretString` wraps every `Credential`
field; `expose_secret()` is called only at the narrow concrete-provider HTTP
boundary and the protected keychain-serialization boundary. Serialized JSON
and intermediate envelope fields are zeroized after the backend call, and
secret values are zeroized on drop. Secrets are never formatted into loggable
strings; `doctor` and `--list-models` show only `display_source`. Raw
credential values are not registered as `SecretRedactor` patterns (avoids
expanding the secret's in-memory footprint; Phase 7 redaction handles
transport-level redaction). This scope covers the store/resolver boundary only;
an end-to-end `SecretString`-through-provider-construction refactor is a
deferred follow-up, not Phase 14.

The existing `provider_collection::SecretKey` remains the redacted wrapper for
legacy `AuthDescriptor::StaticApiKey`; Phase 14 does not replace it.
`Credential` uses `SecretString`; T2's `OAuthCredential` and `ResolvedAuth` do
the same. The only bridges to legacy strings are concrete provider HTTP
construction and protected keychain serialization. This explicit coexistence
is the scope cap, not an intermediate end-to-end secret refactor.

**Two-entry persistence.** The non-secret credential-kind marker and protected
credential envelope are separate keychain entries. Writes update the marker
first and the envelope second; this is a fail-closed, retry-recoverable
protocol, not an atomic transaction. A reader during a kind-change transition
receives a typed wrong-kind/corrupt-store error without env fallback. A
second-step failure may leave a marker-only state with the same fail-closed
behavior, and a later successful write repairs it by rewriting both entries.

Redaction evidence is mechanical: under an injected temporary user-config
root, only the secret-free `opi/credential.lock` may appear outside the fake
keyring backend. Tests recursively scan that root plus captured doctor/model
listing stdout, stderr, diagnostics, and formatted errors to prove seeded API,
access, and refresh secrets and the serialized keychain envelope are absent.

**Crate selection (verified).** `keyring-core` (the `keyring` crate is now a
samples crate), `fs4` (rustix-based, async-capable, no `unsafe`; preferred over
`fd-lock` which uses `unsafe` on Windows and conflicts with the avoid-unsafe
posture; `fs2` is stale), `secrecy` (RustCrypto `Zeroizing`). All enter through
`[workspace.dependencies]`.

**Scope boundary.** T1 delivers `Credential`, the store/probe/resolver
primitives, the redacted precomputed probe-state seam, and the lock. It may
persist and probe the OAuth credential envelope, including `base_url`, but it
does not define `OAuthCredential`/`ResolvedAuth` or implement login, logout,
refresh, presenters, or provider compatibility profiles. Those are T2, built
on T1's locked `write`/`delete`.

### T2 — OAuth architecture and per-request auth re-resolution

**Routing.** The approved Anthropic Messages, Copilot-compatible Chat, and
Codex-compatible Responses paths hold an injected `Arc<dyn AuthResolver>` from
`opi-ai`. `ResolvedAuth` carries the auth scheme/secret plus the optional
non-secret base URL and account id needed by the provider's HTTP boundary:

```rust
pub enum AuthScheme {
    ApiKey,
    Bearer,
}

pub struct ResolvedAuth {
    pub scheme: AuthScheme,
    pub secret: SecretString,
    pub base_url: Option<String>,
    pub account_id: Option<String>,
}

pub trait AuthResolver: Send + Sync {
    fn resolve(&self) -> BoxAuthFuture<'_, Result<ResolvedAuth, ProviderError>>;
}
```

`opi-coding-agent` owns the concrete resolver:

```rust
enum AuthSource {
    Baked(SecretString),
    Store {
        resolver: Arc<CredentialResolver>,
        provider_id: String,
    },
    EnvOAuthToken {
        env_var: String,
    },
}
```

`AuthSource` implements `AuthResolver`: `Baked` returns directly; `Store`
calls the resolver (including locked refresh when the token is near expiry);
`EnvOAuthToken` reads the non-refreshable environment token. Existing direct
`opi-ai` constructors wrap their fixed key in a no-IO `StaticAuthResolver`, so
library users retain a small construction path without moving env or keychain
access into `opi-ai`.

`Provider::stream()` remains synchronous and returns `EventStream`; auth
resolution occurs inside that returned asynchronous stream immediately before
the HTTP request. The provider alone calls `expose_secret()` at the HTTP
boundary. `ProviderCollection` stays off the live path, and the provider types
do not become generic over the resolver.

**OAuth providers.** Complete Phase 14 OAuth coverage for Anthropic, GitHub
Copilot, and OpenAI Codex. The trait handles Anthropic PKCE, Copilot
device-code, and both Codex Browser PKCE and Codex Device Code. The corrective
source also requires Copilot model/wire catalog parity with the reviewed
pi-0.80.6 snapshot.

The provider mapping is exact rather than an auth-header-only approximation:

- Anthropic OAuth selects Bearer auth and the required OAuth beta header while
  API-key construction retains `x-api-key`.
- GitHub Copilot device login exchanges the GitHub token for the short-lived
  Copilot token and derives the per-credential API base URL, including
  enterprise hosts. One `github-copilot` provider/catalog routes models through
  Anthropic Messages, OpenAI Chat Completions, or OpenAI Responses according to
  each model's exact wire identity.
- OpenAI Codex uses a dedicated `OpenAiCodexResponsesProvider`, targets
  `https://chatgpt.com/backend-api/codex/responses`, requires the persisted
  `chatgpt-account-id`, and owns its request body, headers, and stream mapping.
  Shared low-level Responses parsing helpers are allowed; construction through
  `OpenAiResponsesProvider` compatibility flags is not.

Mock request capture must assert these URLs and headers. A test that checks only
that a Bearer token reached an HTTP request is not sufficient evidence for
Anthropic, Copilot, or Codex product acceptance.

**Trait (opi-ai).** `OAuthProvider` and `OAuthCredential` are abstract; the
three impls and an `OAuthProviderRegistry` (`register_oauth_provider` with the
three built-ins) live in `opi-coding-agent`. Because the registry is
heterogeneous, `OAuthProvider::{login, refresh}` return `BoxAuthFuture` and are
object-safe; no `async-trait` dependency is needed. Flow specifics (PKCE
callback server vs device-code polling) live inside each implementation's
`login()`; the trait is flow-agnostic. `LoginPresenter::await_manual_code`
uses the same boxed-future convention when called through `dyn LoginPresenter`.
Only the Anthropic and Codex Browser PKCE flows call it; Copilot calls
`present_device_code` and polls without requesting manual input.

```rust
pub struct OAuthCredential {
    pub access: SecretString,
    pub refresh: SecretString,
    pub expires_at: Option<OffsetDateTime>,
    pub base_url: Option<String>,  // Copilot enterprise
    pub account_id: Option<String>, // OpenAI Codex
}
```

**Refresh (double-checked locking).** Fast path reads the keychain without a
lock; if `now + 5min < expires_at`, return (common case, zero lock cost).
Refresh path: acquire the `fs4` lock, re-read (double-check), and if still
expired perform `oauth_provider.refresh()` HTTP under the lock, write, release.
A 5-minute skew and re-read-after-failure apply. The refresh-HTTP timeout plus
T1's short lock timeout prevent a hung-refresh deadlock. Lock-during-HTTP
prevents refresh-token-rotation double-refresh races.

**Revocation and session-interaction (spec-mandated).** `/logout` is
`store.delete` (T1, locked). External revocation: auth-invalid is classified
non-retryable by adding explicit `ProviderError::CredentialRevoked` and
`ProviderError::CredentialNeeded` variants and mapping both through
`ProviderError::is_retryable`, the provider diagnostic taxonomy, `AgentError`,
and the coding harness without string matching. A revoked credential produces
the `CredentialRevoked` diagnostic, ends the turn, and never triggers login or
retry.

When a credential is absent before any provider output, interactive mode emits
typed `CredentialNeeded`, runs the user-initiated login presenter, and on
success retries the same pending turn without appending a duplicate user
message. Cancellation or login failure leaves that turn failed and persists no
credential. RPC/JSON/non-interactive modes emit typed `CredentialNeeded` plus
the provider id and explicit `/login <provider>` remediation, then fail without
prompting or blocking. They do not manufacture a transient OAuth authorization
URL by starting a login flow. There is no auto-relogin mid-stream: a 401 is
`CredentialRevoked`, stops the turn, and requires a later explicit login.

**LoginPresenter (opi-ai trait).** `select_login_method`,
`present_auth_url`, `present_device_code`, `await_manual_code`,
`notify_success`, and `notify_failure`. The production `TuiLoginPresenter`
lives in `opi-coding-agent`. Anthropic Browser PKCE and Codex Browser PKCE use
`await_manual_code` for fallback. Copilot and Codex Device Code use
`present_device_code` and never request paste-back. RPC/JSON/non-interactive
credential-needed handling is a typed diagnostic path, not an unused presenter
implementation and not a login-flow trigger.

Task 14.2 creates the production `/login` and `/logout` branches in
`run_interactive_tui`; they are not pre-existing call sites. PKCE tests verify
S256 challenge/state matching, loopback-only callback binding, timeout,
mismatched-state rejection, and callback/manual-input cancellation races.
Device-code tests verify pending, slow-down, denial, expiry, timeout, and
cancellation behavior. None of these paths log authorization codes, access
tokens, refresh tokens, or keychain payloads.

Auth-invalid mock responses are captured separately for factory-built
Anthropic OAuth, all three `github-copilot` routes, and the dedicated
`openai-codex` route. Each maps to typed non-retryable `CredentialRevoked`,
emits no retry or login call, and performs no request after the auth-invalid
response.

**ANTHROPIC_OAUTH_TOKEN.** Recognized as `AuthSource::EnvOAuthToken` (no refresh
token, non-refreshable, use until 401 then re-login), with precedence over
`ANTHROPIC_API_KEY`. In Phase 14.

**Per-call override.** Out of Phase 14 (fog). No multi-tenant use case;
`AuthSource` per-stream resolution suffices; T3 `Request` scalars cover
headers/timeout; pi's per-call `apiKey` override is deferred.

**Spec entry-condition check (`opi-spec.md:1641`).** All seven OAuth entry
items are designed by Phase 14:

| Mandated item | Designed in |
|---|---|
| credential store | T1 |
| redaction | T1 D5 |
| doctor | T1 (`CredentialProbe` store arm) |
| session-interaction | T2 D5 |
| login UX | T2 D3 + D6 |
| refresh | T2 D4 |
| revocation | T2 D5 |

Phase 14 OAuth clears its own spec entry condition.

### T3 — opi-ai Request enrichment

`Request` has exactly ten fields today (`provider.rs:30-41`: `model`, `system`,
`messages`, `tools`, `max_tokens`, `temperature`, `thinking`, `stop_sequences`,
`metadata`, `cancel`) and no per-request knobs. Scope decomposes into five
sub-decisions; 3a/3b/3c/3e are in Phase 14, 3d is deferred.

**3a — Request scalars.** Four wire-consumed additive fields, default None/empty:
`timeout: Option<Duration>`, `extra_headers: HeaderMap`,
`cache_retention: Option<CacheRetention>` (feeds 3b), and
`session_id: Option<String>`. `CacheRetention` has `Disabled`, `Short`, and
`Long`; `None` means the provider default (short where prompt caching exists),
while `Some(Disabled)` suppresses session-affinity and cache-retention wire
fields. `timeout` coexists with the existing `CancellationToken` via a `select`
race against `tokio::time::sleep`. `maxRetries`/`maxRetryDelay` are not on
`Request`; retry policy is `opi-agent`'s. `extra_headers` reaches provider HTTP
requests but cannot replace provider-managed authentication, preserving T2's
no-per-call-credential-override decision.

Phase 14 does not add config keys or a coding-harness setter for `timeout`,
`extra_headers`, or `cache_retention`; they are public `opi-ai` request
substrate exercised through concrete provider wire captures. Only `session_id`
has a Phase 14 production producer and traverses the harness/agent boundary.

**Session-affinity production path.** pi forwards the active agent session id
into stream options for provider prompt caching. opi follows the same semantic
path rather than inventing a universal tracing header:

1. after `SessionCoordinator` is created or opened, `CodingHarness` calls a new
   `Agent::set_session_id(Option<String>)`;
2. resume and fork update that value when the active session changes;
3. `Agent` copies it through `AgentLoopContext` into every `Request` constructed
   by `agent_loop`;
4. providers apply only their documented, compatibility-gated cache-affinity
   mapping.

This is the narrow T3 exception to the statement that `opi-agent` is unchanged:
`opi-agent` carries an opaque string but owns no session persistence, provider
construction, auth resolution, or provider-specific mapping.

**Provider-specific `session_id` mapping (pi-0.80.6 aligned).** Mapping occurs
only when `session_id` is non-empty and caching is not explicitly disabled:

- Direct OpenAI Chat Completions emits a top-level `prompt_cache_key`. The
  request's dynamic session id wins; the existing `CompatConfig::cache_key`
  remains the fallback when the request has no session id.
- OpenAI-compatible Chat profiles do not receive session headers by default.
  A new `CompatConfig::send_session_affinity_headers` flag (default `false`)
  emits the pi-aligned `session_id`, `x-client-request-id`, and
  `x-session-affinity` headers when enabled. Existing explicit `cache_key`
  behavior remains separate.
- Direct OpenAI Responses emits `prompt_cache_key` plus
  `x-client-request-id`; `ResponsesConfig::send_session_id_header` controls the
  `session_id` header and defaults `true` for the built-in direct profile.
  Custom/proxy profiles must opt in.
- The dedicated OpenAI Codex Responses wire emits the Codex spelling
  `session-id` plus `x-client-request-id`; standard/custom Responses endpoints
  never infer Codex behavior.
- The official Anthropic endpoint emits no session header. Its prompt caching
  is controlled by the 3b `cache_control` markers. A future Anthropic-compatible
  profile may add an explicit `x-session-affinity` compatibility flag, but it
  is not inferred in Phase 14.
- Other providers ignore `session_id` unless a reviewed compatibility mapping
  is added.

OpenAI `prompt_cache_key` values are clamped to 64 Unicode scalar values, as in
pi. Header values are constructed with `HeaderValue::from_str`; invalid values
return `ProviderError::RequestFailed` and never panic. Tests trace a real
`SessionCoordinator` id through harness → agent loop → mock provider, cover
resume/fork replacement, capture the exact OpenAI bodies/headers, and prove the
Anthropic and default-compatible negative cases without live network calls.

**3b — Anthropic cache_control + ModelCapabilities.** The existing
`opi_ai::registry::ModelCapabilities` becomes `#[non_exhaustive]`, gains exactly
`supports_cache_control` and `supports_long_cache_retention`, and becomes the
single `capabilities` field on `ModelInfo`. Its existing
`context_window`, `max_output_tokens`, `supports_images`,
`supports_streaming`, and `supports_thinking` fields move with it; the flattened
duplicates are removed. A public constructor plus `Default`/builder methods
allow downstream 0.x consumers to construct the non-exhaustive type.
`ProviderRegistry::capabilities`, `ProviderCollection::capabilities`, built-in
models, config-derived models, custom-provider registration, and every
workspace `ModelInfo` literal migrate to the nested value. This is not a
per-provider
`AnthropicCompatConfig`; cache_control is a model capability, not a wire-compat
quirk. The Anthropic provider checks `model.capabilities.supports_cache_control`
and emits `cache_control: {type: ephemeral, ttl}` on the system prompt, the last
user/assistant text, and the last tool definition (pi pattern); `ttl='1h'` when
`supports_long_cache_retention && request.cache_retention == Long`, else default
ephemeral. Built-in Anthropic `ModelInfo` carry known capabilities; custom and
unknown models default off (safe — no markers emitted). OpenAI prompt-cache
affinity follows the provider-specific 3a mapping; it is not modeled as an
Anthropic compatibility quirk or as a global model capability.

The reviewed built-in direct-Anthropic matrix is explicit: the current
`claude-sonnet-4-5-20250514`, `claude-opus-4-20250514`, and
`claude-haiku-4-5-20250514` entries support cache control and long retention;
custom/unknown entries default both fields to false. An external-consumer
compile test constructs the non-exhaustive type through its public
constructor/default/builders, and a migration guard proves the flattened
`ModelInfo` capability fields no longer exist in workspace literals.

**3c — Usage / CostBreakdown.** `Usage` (`stream.rs:33`) remains non-`Copy` and
gains `cache_write_1h_tokens: Option<u64>` and
`reasoning_tokens: Option<u64>`. `cache_write_1h_tokens` is a subset of
`cache_write_tokens`; `reasoning_tokens` is a subset of `output_tokens`.
`total_tokens` therefore continues to count the parent buckets exactly once.
`CostBreakdown` remains `Copy` and retains its existing lines. `calculate_cost`
charges the short-cache remainder at `cache_write_cost_per_mtok`, the 1h subset
at twice the input rate, and reasoning through the already-inclusive output
bucket; it adds no duplicate reasoning or 1h line and stores no cost on
messages. Anthropic maps the 1h split; OpenAI Chat and Responses map reasoning
when present. `CumulativeUsage`, session resume reconstruction, and session cost
summaries preserve the 1h subset so aggregate cost remains correct, while
missing breakdowns remain `None`.

Provider mappers reject malformed upstream usage where either optional child
exceeds its parent through the existing non-retryable stream/response error
path and emit no invalid `Usage` event; zero, equality, and absence are valid.
Unknown model pricing means the coding-agent session `cost_summary` remains
`None`; `calculate_cost` itself always uses the supplied `Pricing`. Resume
coverage extends only the existing usage aggregation/persistence path: it does
not change the session schema version, branch/context selection, or context
reconstruction API.

**3e — refresh_models.** The object-safe `Provider` trait gains
`fn refresh_models(&self) -> BoxAuthFuture<'_, Result<Option<Vec<ModelInfo>>,
ProviderError>>` with a default boxed future returning `Ok(None)` for static
providers. Dynamic providers return `Ok(Some(models))` or an explicit error.
`ProviderCollection::refresh(&mut self)` collects results in deterministic
provider-id order, leaves the last-known catalogs unchanged if any provider
fails, and atomically replaces the successful batch in a registry-owned dynamic
catalog layer only after every refresh succeeds. Repeated refreshes replace,
rather than append to, prior dynamic results. A native async trait method is not
used because providers are stored behind `dyn Provider`.

This is deliberately substrate-only in Phase 14: no CLI, doctor, RPC, TUI, or
startup path invokes refresh yet, and it owns no product acceptance scenario.
Mock trait/collection tests are substrate evidence. A later phase must add a
real dynamic provider and user/API trigger before claiming runtime refresh.

**3d — onPayload/onResponse streaming hooks — deferred.** Per-chunk streaming
interception needs a separate Rust-native design (closures cannot live on a
serde-derived `Request`), and is distinct from Phase 17 T14's turn-level
provider hook. Deferred to fog.

## 2026-07-17 pi-0.80.6 Alignment Revision

The corrective source adds one provider/wire/catalog workstream after the
completed T1-T3 implementation. It does not rewrite the shipped history above,
but its current contracts supersede the historical Codex compatibility-profile
and `api-map` deferral language.

### Exact wire identity and mapped providers

The existing `ApiKind` remains the normalized assistant-message source
classification. A separate non-exhaustive `WireApi` identifies the exact
request wire. Every `ModelInfo` declares one `wire_api`; the initial set is
`anthropic-messages`, `openai-completions`, `openai-responses`,
`openai-codex-responses`, `google-generative-ai`, `google-vertex`,
`bedrock-converse-stream`, and `azure-openai-completions`.

`ApiMappedProvider` exposes one provider id and catalog and holds a checked
`WireApi -> Provider` route map. It resolves the requested model in its own
catalog before dispatch. Unknown models, missing routes, and wire/compat
mismatches fail with typed non-retryable errors before network IO. All routes
for one mapped provider receive the same `Arc<dyn AuthResolver>`, provider id,
and default endpoint.

### Model metadata

`ModelInfo` is the catalog source of truth for:

- exact wire identity and capabilities;
- `off`, `minimal`, `low`, `medium`, `high`, `xhigh`, and `max` thinking-level
  mappings, including explicitly unsupported levels;
- wire-specific compatibility metadata; and
- optional base pricing plus deterministic input-token threshold tiers.

The existing strict Usage subset semantics remain unchanged. Pricing-tier
selection chooses the applicable `Pricing`; it does not add cost lines or
double-count cache/reasoning subsets. `ThinkingConfig` carries the selected
level alongside the existing enabled/budget fields so the wire can apply the
model map. New thinking levels are additive session event values and do not
change the session schema version or branch/context reconstruction model.

### User-configured multi-wire providers

`[providers.custom.<id>]` accepts one provider-level credential source,
authentication scheme, default `base_url`, proxy, headers, and optional default
`api`. Each model must inherit or declare an exact API and may override
`base_url`, capabilities, thinking map, compatibility metadata, and pricing.
The TOML surface permits Anthropic Messages, OpenAI Chat Completions, and
OpenAI Responses. The subscription-only Codex wire is built-in-only.

The existing `[providers.openai_compatible]` table remains the single-wire
OpenAI Completions shorthand but lowers into the same mapped-provider
construction path. Unknown/disabled wires, invalid compatibility metadata,
duplicate models, missing routes, invalid price tiers, and reserved auth
headers fail during configuration/build before a request.

### Built-in parity targets

- Provider ids become `github-copilot` and `openai-codex`. The development-only
  `copilot` and `codex` ids, config values, and keychain account keys receive
  no aliases or migration; users re-login.
- `github-copilot` uses one pi-0.80.6 catalog across Anthropic Messages
  (`/v1/messages`), OpenAI Completions (`/chat/completions`), and OpenAI
  Responses (`/responses`). Every stream re-resolves both token and enterprise
  base URL.
- `openai-codex` uses the reviewed pi-0.80.6 catalog and the dedicated
  `openai-codex-responses` implementation at
  `https://chatgpt.com/backend-api/codex/responses`.
- Codex login offers Browser (default) and Device Code (headless). Browser is
  PKCE callback/manual paste; Device Code uses OpenAI's device authorization,
  polling, and authorization-code exchange endpoints without paste-back.

The checked-in catalog fixture records pi version and SHA-256 and is the
offline acceptance oracle. `--list-models` shows that static snapshot without
reading OAuth secrets or calling Copilot entitlement/model-enable endpoints.

Detailed task ownership, error behavior, test commands, and final exit rules
are normative in the corrective source's 2026-07-17 alignment revision.

## Sequencing

T1 is the substrate. T2 builds on T1's locked `write`/`delete` for login,
logout, and refresh; it must follow T1. T3 3a's generic request fields are
independent, but its Codex-specific session mapping follows the dedicated
Codex provider/wire work in the corrective source. T3 3b follows 3a because cache markers
consume `CacheRetention`; 3c usage/cost accounting and the 3e refresh substrate are
independent and may land before 3a. The final documentation/guard task follows
all six implementation tasks. Phase 14 has no hard dependency on Phase 15 or 16;
Phase 16 T9 (read-tool inline image) assumes the 14 -> 15 -> 16 sequence for its
`FileOperations` substrate but does not depend on auth.

## Success and Exit Criteria

Phase 14 exits only when all of the following are independently traced to the
named production path or, for SC7, explicitly classified substrate evidence:

1. **Credential storage and probes.** Fake keychain/env backends prove locked
   read/write/delete/probe behavior, headless API-key fallback, explicit doctor
   and `--list-models` store arms, redaction, corrupt/backend-unavailable
   diagnostics, and the exact temp-root/output scan proving no plaintext
   credential artifact or formatted secret.
2. **OAuth product flows.** Anthropic PKCE, Copilot device-code, Codex Browser
   PKCE, and Codex Device Code complete through the production registry and
   `/login` command; method selection, flow-specific manual behavior,
   cancellation, failure, `/logout`, provider-specific URL/header capture, and
   keychain-required persistence are covered without live calls.
3. **Live auth and session interaction.** Every approved concrete provider
   resolves auth inside each returned stream. Typed `CredentialNeeded` can
   resume the same interactive turn only after successful user-initiated login;
   RPC/JSON/non-interactive fail without blocking, and typed
   `CredentialRevoked` is non-retryable and never auto-relogs in across
   Anthropic, all three GitHub Copilot wire routes, and the dedicated OpenAI
   Codex provider.
4. **Request and session affinity.** Timeout, extra headers, and cache retention
   are proven as public Request-to-concrete-provider substrate with no claimed
   Phase 14 config/harness producer. The active session id traverses the real
   harness/agent/provider path; exact standard OpenAI, compatible Chat, standard
   Responses, Codex Responses, and negative Anthropic/default-compatible
   mappings are captured.
5. **Capabilities and cache markers.** The existing capabilities type is the
   single nested `ModelInfo` capability representation, and Anthropic markers
   appear only at the exact capable-model positions/TTL with custom/unknown
   defaults off.
6. **Usage and cost.** Anthropic 1h cache-write and OpenAI Chat/Responses
   reasoning breakdowns preserve subset semantics through provider mapping,
   cumulative session accounting, resume, and cost calculation without double
   counting or message-stored cost.
7. **Dynamic refresh substrate.** Object-safe mixed static/dynamic mock
   providers prove deterministic, atomic replacement and error behavior. This
   criterion is substrate-only and does not claim a production trigger.
8. **Documentation and guards.** Public API/provider/command documentation and
   localized counterparts touched by each task describe the shipped behavior;
   `CHANGELOG.md` records the public 0.x breaks and additions; forbidden-scope
   guards continue to reject every Phase 14 Non-Goal. Subprocess/dispatcher
   tests execute the relevant CLI/TUI help and credential-needed remediation;
   source-text synchronization alone is not runtime-help evidence.

The phase-exit evaluator must rebuild these eight criteria and all Non-Goals
from this file, run every owning task scenario, preserve runtime artifacts, and
refuse archive if a product criterion has only helper/mock substrate evidence.
Default tests use fake stores and mock HTTP only; no real keychain, credential,
provider network, or user runtime directory is accessed.

## Residuals / follow-ups

- **Per-call credential override** (pi `ApiStreamOptions`) — fog; re-sharpen
  when a multi-tenant or extension driver appears.
- **onPayload/onResponse streaming hooks** (T3 3d) — fog; re-sharpen when a
  streaming-observation driver appears. Distinct from Phase 17 T14's turn-level
  `before_provider_request` hook.
- **End-to-end `SecretString` through concrete-provider construction** —
  deferred follow-up (T1 D5 scope cap), not Phase 14.
- **§15 roadmap rewrite.** Batched with the Phase 15 and 16 design docs landing.
  Editing `opi-spec.md` triggers the phase4 + phase6 specification-hash ledger
  re-sync plus the live-ledger raw-hash re-sync (per project convention). This
  is a separate, guard-affecting step, not part of authoring this design doc.

## Out of scope (cross-ref map)

- Image-generation (cluster I) internals — fog; depends on the T2 auth seam.
- Broad provider catalogs beyond the reviewed GitHub Copilot and OpenAI Codex
  pi-0.80.6 snapshots remain Future Ecosystem.
- Full pi behavior parity remains out of scope where the corrective source
  records an intentional opi hardening or security boundary.
