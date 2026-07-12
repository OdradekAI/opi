# Phase 14 Provider & Auth Design

Historical note: under the 2026-07-10 roadmap redesign
(`docs/superpowers/plans/2026-07-10-phase-roadmap-redesign-map.md`), Phase 14 is
**Provider & Auth**. The prior Phase 14 (TUI product polish,
`2026-06-24-phase14-tui-product-polish-design.md`) is renumbered to Phase 17.
This doc synthesizes tickets T1 (credential store), T2 (OAuth + per-request
auth), and T3 (opi-ai Request enrichment), all resolved 2026-07-11.

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
  Copilot, and OpenAI Codex, with double-checked-locking token refresh and a
  manual-paste login fallback for headless/no-browser hosts.
- Per-request auth re-resolution on the live run path without moving dispatch
  onto `ProviderCollection` (preserving the Phase 10 boundary).
- Additive `Request` scalars (`timeout`, `extra_headers`, `cache_retention`,
  `session_id`), `Usage` / `CostBreakdown` cache-and-reasoning fields, a
  cross-provider `ModelCapabilities` struct, and a dynamic `refresh_models`
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
- No per-call `apiKey` / `headers` / `env` override (pi `ApiStreamOptions`) —
  deferred to fog (T2 D8); no multi-tenant use case.
- No `onPayload` / `onResponse` streaming hooks (T3 3d) — deferred to fog;
  distinct from Phase 17 T14's turn-level provider hook.
- No `maxRetries` / `maxRetryDelay` on `Request` — retry policy is `opi-agent`'s
  `agent_loop`, not opi-ai wire.
- No end-to-end `SecretString`-through-provider-construction refactor (T1 D5
  scope cap; deferred follow-up).
- No new OAuth providers beyond the three pi ships.
- No changes to session schema, context reconstruction, or the TUI (those belong
  to Phases 13 and 17).

## Relationship to pi

pi persists credentials in a plaintext `auth.json`. opi diverges: the OS
keychain is the primary store for all persisted credentials (API keys and OAuth
tokens), with env-var fallback for API keys on hosts without a keychain daemon.
This is a deliberate security improvement, not parity.

pi has three OAuth providers — Anthropic, GitHub Copilot, OpenAI Codex — using
PKCE authorization-code with a local `127.0.0.1` callback (Anthropic, Codex) and
device-code (Copilot). opi matches all three. Every flow supports a manual
code/URL-paste fallback for headless and SSH hosts, mirroring pi's
`onManualCodeInput`.

pi's live run path routes through `ProviderCollection` per request for auth
re-resolution. opi keeps the live path on `Box<dyn Provider>`
(`agent_loop.rs:118` calls `context.provider.stream(request)` directly) and
instead makes each concrete provider hold an injected `AuthSource` that
re-resolves on each `stream()`. `ProviderCollection` stays off the live path
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
`CredentialSource`, `OAuthProvider`, `OAuthCredential`, `AuthSource`, and
`LoginPresenter` traits/types are defined in `opi-ai` (abstract, no IO/env).
Their concrete implementations — keychain/env/resolver stores, the three OAuth
providers and their registry, the `AuthSource` wiring, and the presenter impls —
live in `opi-coding-agent`. opi-agent is unchanged: the new auth resolution
happens inside each concrete provider's `stream()`, reached through the existing
`Box<dyn Provider>` seam.

## Implementation Priority and Crate Boundaries

| Priority | Scope | Owner | Requirement |
|---|---|---|---|
| P0 | `CredentialStore` trait + `Credential` type | `opi-ai` | Abstract `read`/`write`/`delete`/`probe`; no IO or env access. |
| P0 | `CredentialSource` three-state enum | `opi-ai` | `Present(label)` / `Absent` / `BackendUnavailable(reason)` so doctor distinguishes "missing entry" from "no keychain daemon". |
| P0 | `AuthDescriptor::StoreCredential` variant | `opi-ai` | Additive `{ key, display_source }`; cheap, `Clone`, no secret. New match arms in `doctor`, `dispatch_stream`, `--list-models`. |
| P0 | `KeychainCredentialStore` + `EnvCredentialSource` + `CredentialResolver` + `fs4` global lock | `opi-coding-agent` | `keyring-core` primary; env fallback; single `<user_config_dir>/opi/credential.lock`; acquire-then-re-read. |
| P0 | `OAuthProvider` trait + `OAuthCredential` | `opi-ai` | `id()` / async `login(presenter)` / async `refresh(refresh)`; flow-agnostic; native async (edition 2024, no `async-trait`). |
| P0 | Three OAuth impls + `OAuthProviderRegistry` | `opi-coding-agent` | Anthropic (PKCE + `127.0.0.1` callback), Copilot (device-code), Codex (PKCE); `register_oauth_provider` with the three built-ins. |
| P0 | `AuthSource` enum injected per concrete provider | `opi-coding-agent` | `Baked(SecretString)` / `Store(Arc<dyn CredentialStore>, provider_id)` / `EnvOAuthToken`; resolved per `stream()`. |
| P0 | `Request` enrichment scalars | `opi-ai` | `timeout: Option<Duration>`, `extra_headers: HeaderMap`, `cache_retention: Option<CacheRetention>`, `session_id: Option<String>` (additive, default None/empty). |
| P0 | `Usage` / `CostBreakdown` cache + reasoning fields | `opi-ai` | `cache_write_1h_tokens`, `reasoning_tokens` (Option); preserve `Copy` and the compute-separately cost model. |
| P0 | `ModelCapabilities` struct on `ModelInfo` | `opi-ai` | `#[non_exhaustive]`; `supports_cache_control`, `supports_long_cache_retention`, `supports_thinking`, ...; default off for custom/unknown models. |
| P0 | Anthropic `cache_control` markers | `opi-ai` (anthropic provider) | Emit `{type:ephemeral,ttl}` on system + last user/assistant text + last tool def when `supports_cache_control`; `ttl='1h'` when `supports_long_cache_retention && cache_retention==Long`. |
| P1 | `refresh_models` on `Provider` trait | `opi-ai` | Async `fn refresh_models(&self) -> Option<Vec<ModelInfo>>`, default `None`; `ProviderCollection::refresh` (`provider_collection.rs:374`) fans out to `Some`-returning providers. |
| P1 | `LoginPresenter` trait + impls | `opi-ai` (trait) / `opi-coding-agent` (impls) | `TuiLoginPresenter` / `RpcLoginPresenter` / `NonInteractiveLoginPresenter`; manual-paste fallback mandatory on every flow. |
| P1 | `doctor` `CredentialProbe` store arm | `opi-coding-agent` | Extend the `EnvApiKey` / `StaticApiKey` arms at `doctor.rs:619-625` to report `store.probe` presence without reading the secret. |

Phase 14 must not satisfy acceptance with the abstract traits alone. Each P0
item needs a production path from config through `provider_factory` into the
concrete provider's `stream()`, exercised by `MockProvider`-style integration
tests (never a real LLM API).

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
        expires_at: Option<DateTime<Utc>>,
    },
}

#[async_trait]  // conceptually; native async fn in trait on edition 2024
pub trait CredentialStore: Send + Sync {
    async fn read(&self, provider_id: &str) -> Option<Credential>;
    async fn write(&self, provider_id: &str, cred: &Credential) -> Result<()>;
    async fn delete(&self, provider_id: &str) -> Result<()>;
    async fn probe(&self, provider_id: &str) -> CredentialSource;
}
```

The methods are async for forward-compatibility with T2's refresh-on-read.

**Impls (opi-coding-agent).** `KeychainCredentialStore` (via `keyring-core`),
`EnvCredentialSource`, and a composing `CredentialResolver` own all IO and env
access. This is where the credential file lock lives, not in the trait.

**AuthDescriptor (opi-ai).** Additive variant, no secret:

```rust
AuthDescriptor::StoreCredential { key: String, display_source: String }
```

`AuthDescriptor` is `#[non_exhaustive]` (`provider_collection.rs:98`), so the
variant is additive; every match already carries a wildcard arm. Consumed in
non-live paths only: `doctor`'s `CredentialProbe` (`doctor.rs:619-625`, via
`store.probe`), `dispatch_stream`'s status gate (`provider_collection.rs:337`),
and `--list-models` metadata. The three-state `CredentialSource` lets `doctor`
distinguish a missing entry from an unavailable keychain backend.

**Locking.** A single global `fs4` advisory lock at
`<user_config_dir>/opi/credential.lock` (the lock file holds no secret — it is
pure coordination, because the OS keychain has no cross-"read → refresh → write"
transaction). The exclusive lock wraps every store mutation (`write`, `delete`,
and OAuth refresh in T2), applied uniformly with acquire-then-re-read (pi's
`auth-storage.ts:505-516` pattern: concurrent opi instances share one op), plus
a short timeout and diagnostic on contention rather than an infinite block.

**In-memory and redaction.** `secrecy::SecretString` wraps every `Credential`
field; `expose_secret()` is called only at the narrow concrete-provider HTTP
boundary and the value is zeroized on drop. Secrets are never formatted into
loggable strings; `doctor` and `--list-models` show only `display_source`. Raw
credential values are not registered as `SecretRedactor` patterns (avoids
expanding the secret's in-memory footprint; Phase 7 redaction handles
transport-level redaction). This scope covers the store/resolver boundary only;
an end-to-end `SecretString`-through-provider-construction refactor is a
deferred follow-up, not Phase 14.

**Crate selection (verified).** `keyring-core` (the `keyring` crate is now a
samples crate), `fs4` (rustix-based, async-capable, no `unsafe`; preferred over
`fd-lock` which uses `unsafe` on Windows and conflicts with the avoid-unsafe
posture; `fs2` is stale), `secrecy` (RustCrypto `Zeroizing`). All enter through
`[workspace.dependencies]`.

**Scope boundary.** T1 delivers the store primitives (`read`/`write`/`delete`/
`probe`) and the lock. The `/login` / `/logout` command UX and the OAuth flow
are T2, built on T1's locked `write`/`delete`.

### T2 — OAuth architecture and per-request auth re-resolution

**Routing.** Each concrete provider holds an injected `AuthSource` enum and
re-resolves it on each `stream()`:

```rust
enum AuthSource {
    Baked(SecretString),
    Store(Arc<dyn CredentialStore>, provider_id),
    EnvOAuthToken,
}
```

`Baked` returns directly; `Store` calls `store.read` (including locked refresh
when the token is near expiry); `EnvOAuthToken` uses the env token. The
`Provider` trait and the live path are unchanged — `ProviderCollection` stays
off the live path (Phase 10 boundary preserved); refresh and locking are
centralized in the store. This resolves the live-path routing fork T1 deferred.

**OAuth providers.** Full pi parity for all three: Anthropic, GitHub Copilot,
OpenAI Codex. The trait handles PKCE authorization-code (Anthropic, Codex) and
device-code (Copilot) from the start.

**Trait (opi-ai).** `OAuthProvider` and `OAuthCredential` are abstract; the
three impls and an `OAuthProviderRegistry` (`register_oauth_provider` with the
three built-ins) live in `opi-coding-agent`. Native async fn in trait (edition
2024, no `async-trait` crate). Methods: `id()`, async `login(presenter) ->
OAuthCredential`, async `refresh(refresh) -> OAuthCredential`. Flow specifics
(PKCE callback server vs device-code polling) live inside each impl's `login()`;
the trait is flow-agnostic.

```rust
pub struct OAuthCredential {
    pub access: SecretString,
    pub refresh: SecretString,
    pub expires_at: Option<DateTime<Utc>>,
    pub base_url: Option<String>,  // Copilot enterprise
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
non-retryable — extend `ProviderError::is_retryable` (`provider.rs:152`,
currently `RateLimited` / `Timeout` / `Network`) — surfacing a
`CredentialRevoked` diagnostic; the stream errors out and the session stops
gracefully. When a credential is needed mid-run, interactive (TUI) mode emits
`CredentialNeeded`, the `LoginPresenter` prompts `/login`, and the session
resumes on success (cancelable); non-interactive (RPC/JSON) mode emits a
`CredentialNeeded` diagnostic and login URL and fails the turn (no auto-prompt,
no block). There is no auto-relogin mid-stream — a 401 stops the turn and
prompts re-login.

**LoginPresenter (opi-ai trait).** `present_auth_url`, `present_device_code`,
`await_manual_code` (the manual-paste fallback), `notify_success`,
`notify_failure`. Implementations `TuiLoginPresenter` / `RpcLoginPresenter` /
`NonInteractiveLoginPresenter` live in `opi-coding-agent`. Every OAuth flow
supports the manual fallback (headless/SSH/no-browser).

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

**3a — Request scalars.** Four wire-consumed additive fields, default None/empty,
backward-compatible: `timeout: Option<Duration>`, `extra_headers: HeaderMap`,
`cache_retention: Option<CacheRetention>` (feeds 3b), `session_id: Option<String>`.
`timeout` coexists with the existing `CancellationToken` via a `select` race
against `tokio::time::sleep`. `maxRetries`/`maxRetryDelay` are not on `Request`;
retry policy is `opi-agent`'s.

**3b — Anthropic cache_control + ModelCapabilities.** A new cross-provider
`ModelCapabilities` struct (`#[non_exhaustive]`) on `ModelInfo` (`provider.rs:65`)
— `supports_cache_control`, `supports_long_cache_retention`,
`supports_thinking`, and so on. This is not a per-provider
`AnthropicCompatConfig`; cache_control is a model capability, not a wire-compat
quirk. The Anthropic provider checks `model.capabilities.supports_cache_control`
and emits `cache_control: {type: ephemeral, ttl}` on the system prompt, the last
user/assistant text, and the last tool definition (pi pattern); `ttl='1h'` when
`supports_long_cache_retention && request.cache_retention == Long`, else default
ephemeral. Built-in Anthropic `ModelInfo` carry known capabilities; custom and
unknown models default off (safe — no markers emitted). The existing OpenAI
`prompt_cache_key` (`openai_chat.rs`, via `cache_key`) is unchanged.

**3c — Usage / CostBreakdown.** Additive Option fields preserving `Copy` and the
compute-separately cost model (cost via `calculate_cost`, not stored on the
message): `Usage` (`stream.rs:33`) gains `cache_write_1h_tokens: Option<u64>` and
`reasoning_tokens: Option<u64>`; `CostBreakdown` (`stream.rs:185`) gains the
matching cost lines.

**3e — refresh_models.** The `Provider` trait gains
`async fn refresh_models(&self) -> Option<Vec<ModelInfo>>` with a default `None`
(static providers); dynamic providers override. `ProviderCollection::refresh`
(`provider_collection.rs:374`, currently `Ok(())`) fans out to `Some`-returning
providers.

**3d — onPayload/onResponse streaming hooks — deferred.** Per-chunk streaming
interception needs a separate Rust-native design (closures cannot live on a
serde-derived `Request`), and is distinct from Phase 17 T14's turn-level
provider hook. Deferred to fog.

## Sequencing

T1 is the substrate. T2 builds on T1's locked `write`/`delete` for login,
logout, and refresh; it must follow T1. T3 is independent of T1/T2 (it touches
different `opi-ai` surfaces) and can proceed in parallel once the `Request`
scalar additions land. Phase 14 has no hard dependency on Phase 15 or 16;
Phase 16 T9 (read-tool inline image) assumes the 14 -> 15 -> 16 sequence for its
`FileOperations` substrate but does not depend on auth.

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

- Multi-API provider (`api-map`) internals — fog; sharpen after T3 settles the
  `Request` surface and T2 settles per-request auth routing.
- Image-generation (cluster I) internals — fog; depends on the T2 auth seam.
- Broad provider catalog / dedicated Mistral or Codex wires (cluster B) — Future
  Ecosystem.
- Full pi parity (posture B chose strategic gap-closing).
