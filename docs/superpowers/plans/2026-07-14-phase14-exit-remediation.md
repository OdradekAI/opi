# Phase 14 Exit Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close all 20 accepted Phase 14 exit blockers, repair the 29 historical acceptance commands, and rerun phase exit until Phase 14 is archive-eligible without weakening any criterion or Non-Goal.

**Architecture:** Install one target-selected native keyring behind the existing `keyring-core` seam; move doctor, listing, and interactive auth into injectable production orchestrators; keep approved providers constructible with lazy per-stream auth; validate usage subsets before emitting terminal events; and preserve optional usage fields through accumulation and session replay. Tasks 14.8-14.10 are sequential, 14.11 and 14.12 are independent after ledger reinit, and 14.13 is the final convergence gate.

**Tech Stack:** Rust 2024, Tokio, `keyring-core` v1 plus target-native stores, `fs4`, `secrecy`, `wiremock`, ratatui/crossterm, serde JSONL, Cargo workspace tests, PowerShell verification on the host.

## Global Constraints

- Treat `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md` as the corrective source and `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md` as historical context.
- Preserve tasks 14.1-14.7, their commits, passing status, evidence, and `verified_at_commit` values. Only reviewed reinit may repair `acceptance_scenarios`.
- Never delete or hand-edit `.opi-impl-state.json`; invoke the `opi-implement` spec-drift/reinit flow.
- Do not stage or commit the existing workflow relocation: `scripts/exec.workflow.js`, `scripts/phase-exit.workflow.js`, `scripts/plan.workflow.js`, or `.claude/skills/opi-implement/scripts/`.
- Tests must not access a real keychain, OAuth/LLM endpoint, browser, terminal, or user session directory. Use fake stores, `wiremock`, recording terminal controls, `tempfile`, and serialized environment access.
- Add dependency versions only under root `[workspace.dependencies]`; crate manifests consume them with `workspace = true`.
- Follow red-green-refactor for every behavior change. A red test must fail for the intended reason before implementation.
- After each code task, run its focused tests, `cargo fmt --check --all`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- All six corrective tasks keep `evaluator_required = true`.
- Do not commit unless the user has granted commit authorization for the execution session. When authorized, stage only the exact files listed in the task.

---

## Task 0: Register the Corrective Source and Reinitialize the Ledger

**Files:**

- Modify: `.claude/skills/opi-implement/skill.md`
- Modify: `.claude/skills/opi-implement/references/initializer.md`
- Modify: `.claude/skills/opi-implement/references/verification-tiers.md`
- Modify: `.claude/skills/opi-implement/references/verify-engine.md`
- Modify: `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`
- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Tool-owned update after reinit: `docs/snapshots/phase4/opi-impl-state.json`
- Tool-owned update after reinit: `docs/snapshots/phase6/opi-impl-state.json`
- Never hand-edit: `.opi-impl-state.json`

### Step 1: Register the supplemental source

- [ ] Add the corrective design as a second Phase 14 row/source in both reviewed-source lists in `.claude/skills/opi-implement/skill.md`:

```markdown
| 14 | `docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md` |
| 14 | `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md` |
```

- [ ] Update the three verify-engine workflow references in `.claude/skills/opi-implement/skill.md`, `references/initializer.md`, `references/verification-tiers.md`, and `references/verify-engine.md` from `scripts/*.workflow.js` to `.claude/skills/opi-implement/scripts/*.workflow.js`. The workflow relocation itself remains baseline-only and outside this task commit.

- [ ] Add this note immediately below the opening context in the historical design:

```markdown
> Remediation status (2026-07-14): tasks 14.1-14.7 shipped, but phase exit
> returned `not-met-block`. The reviewed corrective source is
> `docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`;
> it adds tasks 14.8-14.13 without rewriting this historical design.
```

- [ ] Change the Phase 14 status in `docs/opi-spec.md` to:

```markdown
Status: implemented; remediation pending. Historical design:
`docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`. Corrective design:
`docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`.
```

- [ ] Apply the exact localized counterpart in `docs/opi-spec.zh.md`:

```markdown
状态：已实现；修复待完成。历史设计：
`docs/superpowers/specs/2026-07-11-phase14-provider-auth-design.md`。修复设计：
`docs/superpowers/specs/2026-07-14-phase14-exit-remediation-design.md`。
```

### Step 2: Repair the 29 historical command records during reviewed reinit

- [ ] Invoke `/opi-implement` and accept its spec-drift path. Reinitialize Phase 14 from the three registered normative files; do not replace or edit the ledger JSON directly.
- [ ] Preserve 14.1-14.7 history and add this exact graph:

```text
14.8 -> 14.9 -> 14.10 -> 14.13
14.11 -----------------> 14.13
14.12 -----------------> 14.13
```

- [ ] Replace the 29 historical verification commands with the command matrix in Appendix A. Those commands all target tests that exist before corrective implementation, so each can be checked before graph confirmation. The new 14.8-14.13 scenarios separately own the missing vertical evidence.
- [ ] For each Appendix A command, confirm exit code `0` and a selected-test count greater than zero. Reject graph confirmation if any command names a nonexistent target or reports `0 passed; 0 failed`.
- [ ] Confirm the reinitialized graph only after the task ownership, dependency graph, `evaluator_required` flags, exact owned-file sets, and new acceptance scenarios match Tasks 1-6 below.

### Step 3: Verify spec and ledger integration

- [ ] Run:

```powershell
cargo test -p opi-coding-agent --test phase4_ledger
cargo test -p opi-coding-agent --test phase6_ledger
cargo test -p opi-coding-agent --test phase14_provider_auth_docs localized_docs_pin_exact_phase14_claims_and_acceptance_rows
```

Expected: each command selects at least one test and passes; the first two snapshot tests accept only tool-generated snapshot/hash changes.

- [ ] Inspect `git diff --check` and `git status --short`. Confirm the four workflow relocation paths remain baseline-only and unstaged.
- [ ] If commit authorization is active, stage only the registered/source/reference/snapshot paths listed for Task 0 that actually changed and commit:

```powershell
git commit -m "docs(phase14): register exit remediation"
```

---

## Task 1: Implement 14.8 Native Keyring and Production Probes

**Files:**

- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/opi-ai/src/credential.rs`
- Modify: `crates/opi-ai/src/anthropic.rs`
- Modify: `crates/opi-ai/src/openai_chat.rs`
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-ai/src/openrouter.rs`
- Modify: `crates/opi-ai/src/mistral.rs`
- Modify: `crates/opi-ai/src/gemini.rs`
- Modify: `crates/opi-ai/src/bedrock/mod.rs`
- Modify: `crates/opi-ai/src/vertex.rs`
- Modify: `crates/opi-coding-agent/Cargo.toml`
- Modify: `crates/opi-coding-agent/src/lib.rs`
- Create: `crates/opi-coding-agent/src/native_keyring.rs`
- Modify: `crates/opi-coding-agent/src/credential_store.rs`
- Modify: `crates/opi-coding-agent/src/doctor.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/src/main.rs`
- Modify: `crates/opi-coding-agent/tests/credential_store.rs`
- Modify: `crates/opi-coding-agent/tests/doctor_cli.rs`
- Modify: `crates/opi-coding-agent/tests/list_models.rs`
- Modify: `crates/opi-coding-agent/tests/provider_factory.rs`

### Step 1: Add failing native-store and resolver tests

- [ ] Add `native_keyring_host_selection_installs_a_default_store` as a serialized unit test in `native_keyring.rs`. Install a `keyring_core` mock only inside the test guard, create an entry, and assert the guard's drop unsets the default. The test must never call a platform store.
- [ ] Add these integration tests to `credential_store.rs`:

```rust
#[tokio::test]
async fn resolver_corrupt_envelope_never_falls_back_to_env() {
    // Seed malformed JSON and inject a non-empty env value.
    // Expect CredentialStoreError::MalformedEnvelope, not an env key.
}

#[tokio::test]
async fn resolver_unknown_envelope_never_falls_back_to_env() {
    // Seed version 999 and inject a non-empty env value.
    // Expect CredentialStoreError::UnknownEnvelope, not an env key.
}

#[tokio::test]
async fn resolver_backend_unavailable_falls_back_to_env_with_diagnostic_bit() {
    // Use FakeKeyringBackend::with_unavailable().
    // Expect ApiKeySource::Env { backend_unavailable: true, .. }.
}
```

- [ ] Run the three resolver tests. Expected red: corrupt/unknown cases currently return the env key.

### Step 2: Preserve backend-unavailable as a typed store error

- [ ] Add this public error variant in `opi-ai/src/credential.rs`:

```rust
/// The native credential service is not reachable on this host.
#[error("credential backend unavailable for '{provider}': {reason}")]
BackendUnavailable { provider: String, reason: String },
```

- [ ] Add a second typed error for a valid envelope of the wrong credential kind:

```rust
/// A provider entry exists, but its credential kind is not valid for this path.
#[error("unexpected credential kind for '{provider}': expected {expected}, found {actual}")]
UnexpectedCredentialKind {
    provider: String,
    expected: &'static str,
    actual: &'static str,
},
```

- [ ] Derive `thiserror::Error` for the coding-agent `BackendError` and give both variants redacted display strings so native-store construction errors can be retained without debug formatting.
- [ ] Change `KeychainCredentialStore::backend_err` to preserve the distinction:

```rust
fn backend_err(&self, provider_id: &str, error: BackendError) -> CredentialStoreError {
    match error {
        BackendError::BackendUnavailable(reason) => CredentialStoreError::BackendUnavailable {
            provider: provider_id.to_owned(),
            reason,
        },
        BackendError::Other(reason) => CredentialStoreError::Backend {
            provider: provider_id.to_owned(),
            reason,
        },
    }
}
```

- [ ] Change the API-key resolver contract and implementation to read once and fall back only on absence/unavailability:

```rust
pub async fn resolve_api_key(
    &self,
    provider_id: &str,
    env_var: &str,
) -> Result<Option<ResolvedApiKey>, CredentialStoreError> {
    match self.store.read(provider_id).await {
        Ok(Some(Credential::ApiKey(value))) => Ok(Some(ResolvedApiKey {
            value,
            source: ApiKeySource::Store,
        })),
        Ok(Some(Credential::OAuthToken { .. })) => {
            Err(CredentialStoreError::UnexpectedCredentialKind {
                provider: provider_id.to_owned(),
                expected: "api_key",
                actual: "oauth_token",
            })
        }
        Ok(None) => Ok(self.env_fallback(env_var, false)),
        Err(CredentialStoreError::BackendUnavailable { .. }) => {
            Ok(self.env_fallback(env_var, true))
        }
        Err(error) => Err(error),
    }
}
```

- [ ] Update all callers to propagate the typed error through `ProviderBuildError::Provider(ProviderError::Config(...))`; do not convert corruption to missing credentials.
- [ ] Rerun the three resolver tests. Expected green.

### Step 3: Install target-native keyring stores

- [ ] Add root workspace dependencies:

```toml
apple-native-keyring-store = "1.0"
windows-native-keyring-store = "1.1"
zbus-secret-service-keyring-store = "1.0"
```

- [ ] Add target-specific `workspace = true` dependencies to `crates/opi-coding-agent/Cargo.toml` for `cfg(target_os = "macos")`, `cfg(target_os = "windows")`, and `cfg(target_os = "linux")`.
- [ ] Implement `native_keyring.rs` with one process-lifetime guard and one cfg-selected constructor:

```rust
pub struct NativeKeyringGuard {
    installed: bool,
}

impl Drop for NativeKeyringGuard {
    fn drop(&mut self) {
        if self.installed {
            keyring_core::unset_default_store();
            self.installed = false;
        }
    }
}

pub fn install_native_keyring() -> Result<NativeKeyringGuard, BackendError> {
    let store = platform_store()?;
    install_store(store)
}

fn install_store(
    store: std::sync::Arc<keyring_core::CredentialStore>,
) -> Result<NativeKeyringGuard, BackendError> {
    keyring_core::set_default_store(store);
    Ok(NativeKeyringGuard { installed: true })
}

#[cfg(target_os = "windows")]
fn platform_store(
) -> Result<std::sync::Arc<keyring_core::CredentialStore>, BackendError> {
    let store: std::sync::Arc<keyring_core::CredentialStore> =
        windows_native_keyring_store::Store::new()
            .map_err(|error| BackendError::BackendUnavailable(error.to_string()))?;
    Ok(store)
}

#[cfg(target_os = "macos")]
fn platform_store(
) -> Result<std::sync::Arc<keyring_core::CredentialStore>, BackendError> {
    let store: std::sync::Arc<keyring_core::CredentialStore> =
        apple_native_keyring_store::keychain::Store::new()
            .map_err(|error| BackendError::BackendUnavailable(error.to_string()))?;
    Ok(store)
}

#[cfg(target_os = "linux")]
fn platform_store(
) -> Result<std::sync::Arc<keyring_core::CredentialStore>, BackendError> {
    let store: std::sync::Arc<keyring_core::CredentialStore> =
        zbus_secret_service_keyring_store::Store::new()
            .map_err(|error| BackendError::BackendUnavailable(error.to_string()))?;
    Ok(store)
}
```

The serialized unit test calls private `install_store(keyring_core::mock::Store::new()?)`; no sample/mock store is allowed in non-test code.

- [ ] Make `KeyringCoreBackend` own the guard or a stable unavailable reason:

```rust
pub struct KeyringCoreBackend {
    _native: Option<NativeKeyringGuard>,
    unavailable_reason: Option<String>,
}

impl KeyringCoreBackend {
    pub fn new() -> Self {
        match install_native_keyring() {
            Ok(guard) => Self { _native: Some(guard), unavailable_reason: None },
            Err(error) => Self {
                _native: None,
                unavailable_reason: Some(error.to_string()),
            },
        }
    }

    fn ensure_available(&self) -> Result<(), BackendError> {
        match &self.unavailable_reason {
            Some(reason) => Err(BackendError::BackendUnavailable(reason.clone())),
            None => Ok(()),
        }
    }
}
```

- [ ] Call `ensure_available()` before each `Entry::new`; map Secret Service connection/daemon failures to `BackendUnavailable` and other platform failures to `Other`.
- [ ] Run the native unit test and existing `keychain_store_reaches_production_construction`. Expected: host selection compiles; test default store is released; no user keychain access occurs.

### Step 4: Build metadata-only model catalogs

- [ ] Add public, side-effect-free `pub fn model_catalog() -> Vec<ModelInfo>` functions in each affected `opi-ai` provider module, and make constructors reuse them rather than duplicating model lists. Move every existing `ModelInfo` entry verbatim from the constructor into the corresponding function; the extraction must not change ids, display names, pricing, limits, or capabilities. Add `anthropic::model_catalog`, `openai_chat::model_catalog`, `openai_responses::model_catalog`, `openrouter::model_catalog`, `mistral::model_catalog`, `gemini::model_catalog`, `bedrock::model_catalog`, and `vertex::model_catalog`. Azure and config-driven compatible profiles derive `ModelInfo` only from their existing config deployment/model lists.

- [ ] Add a direct metadata constructor:

```rust
impl MetadataProvider {
    fn new(id: impl Into<String>, models: Vec<ModelInfo>) -> Self {
        Self { id: id.into(), models }
    }
}
```

- [ ] Replace `require_list_models_api_key` and all listing provider constructors with `build_list_models_metadata(config, provider_id) -> Result<MetadataProvider, ListModelsError>`. It validates proxy/config and reads only the catalog/config model list; it never reads an env value, reads a stored secret, or builds a dispatchable HTTP provider.
- [ ] Gate registration on redacted presence:

```rust
fn listing_auth_available(
    env_name: Option<&str>,
    store_probe: Option<&CredentialSource>,
) -> bool {
    let env_present = env_name
        .and_then(|name| std::env::var_os(name))
        .and_then(|value| value.into_string().ok())
        .is_some_and(|value| !value.trim().is_empty());
    env_present || matches!(store_probe, Some(CredentialSource::Present { .. }))
}
```

Keep the existing silent skip when neither source is present. `BackendUnavailable` alone does not claim a credential.

### Step 5: Add async doctor/list orchestration and route production through it

- [ ] Add one shared probe collector:

```rust
pub async fn collect_credential_probes(
    store: &dyn CredentialStore,
    provider_ids: impl IntoIterator<Item = String>,
) -> HashMap<String, CredentialSource> {
    let mut probes = HashMap::new();
    for provider_id in provider_ids {
        probes.insert(provider_id.clone(), store.probe(&provider_id).await);
    }
    probes
}
```

- [ ] Add `run_doctor_with_store(...)` in `doctor.rs` and `build_collection_for_listing_with_store(...)` in `provider_factory.rs`. Both receive `&dyn CredentialStore`, invoke the collector themselves, and then call existing pure formatting/collection helpers.
- [ ] In `main.rs`, create one `KeychainCredentialStore` for each credential-aware early command, then call the async orchestration inside its runtime. Provider startup continues through `build_provider_bundle`, whose `KeyringCoreBackend::new()` installs the native store before constructing any entry.
- [ ] Add fake-backend production-orchestration ordering tests named `native_keyring_precedes_doctor_orchestration`, `native_keyring_precedes_listing_orchestration`, `native_keyring_precedes_live_provider_construction`, and `native_keyring_precedes_interactive_startup`. Enter through the real doctor, listing, provider-bundle, and interactive startup branches; assert native installation precedes entry creation and the process-lifetime guard releases the default. Keep the private host-selection unit test as supplemental evidence only. No test may access the user keychain.
- [ ] Add production-path tests:

```rust
#[tokio::test]
async fn doctor_store_probe_uses_async_command_orchestration() {
    // Fake Present, Absent, and BackendUnavailable providers.
    // Call run_doctor_with_store, not run_doctor with a hand-built map.
    // Assert redacted source labels and no canary secret.
}

#[tokio::test]
async fn stored_only_credential_lists_models_through_async_orchestration() {
    // Remove ANTHROPIC_API_KEY through the serialized env helper.
    // Seed only fake-store anthropic API key.
    // Call build_collection_for_listing_with_store.
    // Assert anthropic models exist and the canary secret does not appear.
}
```

- [ ] Run:

```powershell
cargo test -p opi-coding-agent --test credential_store
cargo test -p opi-coding-agent --test doctor_cli native_keyring_precedes_doctor_orchestration
cargo test -p opi-coding-agent --test doctor_cli doctor_store_probe_uses_async_command_orchestration
cargo test -p opi-coding-agent --test list_models native_keyring_precedes_listing_orchestration
cargo test -p opi-coding-agent --test list_models stored_only_credential_lists_models_through_async_orchestration
cargo test -p opi-coding-agent --test provider_factory native_keyring_precedes_live_provider_construction
cargo test -p opi-coding-agent --test provider_factory listing_collection_
cargo test -p opi-coding-agent --bin opi native_keyring_precedes_interactive_startup
```

Expected: all pass and every command selects at least one test.

### Step 6: Task gates and commit boundary

- [ ] Run `cargo fmt --check --all` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `git diff --check` and verify no workflow relocation path entered the diff.
- [ ] If authorized, stage only Task 1 files and commit:

```powershell
git commit -m "feat(opi-coding-agent): install native credential stores"
```

---

## Task 2: Implement 14.9 Login/Logout Dispatcher and Persistence

**Files:**

- Modify: `crates/opi-coding-agent/src/lib.rs`
- Create: `crates/opi-coding-agent/src/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: `crates/opi-coding-agent/src/oauth.rs`
- Modify: `crates/opi-coding-agent/tests/oauth_auth.rs`
- Create: `crates/opi-coding-agent/tests/interactive_auth.rs`

### Step 1: Add failing dispatcher tests

- [ ] Add three `interactive_auth.rs` integration tests named `interactive_auth_dispatcher_persists_and_deletes_all_profiles`, `interactive_auth_dispatcher_restores_terminal_on_every_exit`, and `interactive_auth_dispatcher_reports_lock_failure_without_success`. Each calls the production dispatcher with the real `OAuthProviderRegistry`, fake OAuth providers, fake locked store, mock presenter, and recording terminal.

The first test iterates `anthropic`, `copilot`, and `codex`, dispatches `/login`, verifies an OAuth envelope through `CredentialStore::read`, dispatches `/logout`, and verifies `None`. The second covers success, provider failure, presenter failure, cancellation by dropping the dispatcher future, and timeout. Every case asserts exactly `suspend, resume`. The third holds the fake lock past the injected timeout and asserts a redacted typed failure.

- [ ] Run the new test target. Expected red: no production dispatcher module exists.

### Step 2: Extract the production command contract

- [ ] Implement these types in `interactive_auth.rs`:

```rust
pub const AUTH_HELP: &[(&str, &str)] = &[
    ("/login <provider>", "authenticate and persist an OAuth credential"),
    ("/logout <provider>", "delete the persisted credential"),
];

pub trait LoginTerminalControl {
    fn suspend_for_login(&mut self) -> std::io::Result<()>;
    fn resume_after_login(&mut self) -> std::io::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthCommandOutcome {
    NotHandled,
    Usage(String),
    LoggedIn { provider_id: String },
    LoggedOut { provider_id: String },
    Failed { message: String },
}

pub struct AuthCommandServices<'a> {
    pub store: &'a KeychainCredentialStore,
    pub registry: &'a OAuthProviderRegistry,
    pub presenter: &'a dyn LoginPresenter,
}
```

- [ ] Implement an RAII guard whose `Drop` always resumes once:

```rust
struct SuspendedTerminal<'a, T: LoginTerminalControl> {
    terminal: &'a mut T,
    active: bool,
}

impl<'a, T: LoginTerminalControl> SuspendedTerminal<'a, T> {
    fn new(terminal: &'a mut T) -> Result<Self, String> {
        terminal.suspend_for_login().map_err(|error| error.to_string())?;
        Ok(Self { terminal, active: true })
    }
}

impl<T: LoginTerminalControl> Drop for SuspendedTerminal<'_, T> {
    fn drop(&mut self) {
        if self.active {
            let _ = self.terminal.resume_after_login();
            self.active = false;
        }
    }
}
```

- [ ] Implement `dispatch_auth_command` so only `/login` suspends the terminal, both commands use the existing locked store helpers, and raw provider errors are reduced to redacted messages:

```rust
pub async fn dispatch_auth_command<T: LoginTerminalControl>(
    input: &str,
    terminal: &mut T,
    services: AuthCommandServices<'_>,
) -> AuthCommandOutcome
```

Exact parsing rules: `/login` and `/logout` without an id return their usage strings; unknown input returns `NotHandled`; whitespace around the provider id is trimmed; extra arguments remain an unknown provider id and produce `Failed` rather than being silently discarded.

### Step 3: Route the TUI through the dispatcher

- [ ] Move the `Terminal<CrosstermBackend<Stdout>>` control implementation from `interactive.rs` to `interactive_auth.rs`.
- [ ] Remove the inline `/login` and `/logout` branches and the old `with_login_terminal_suspended` helper from `interactive.rs`.
- [ ] Construct `AuthCommandServices` from the same store/registry captured from `ProviderBundle`; call the dispatcher before adding a normal user prompt. Convert each handled outcome to exactly one system message.
- [ ] Keep `oauth::login_oauth` and `logout_credential` as mutation helpers, but change their return type from `Result<(), String>` to `Result<(), ProviderError>` so lock/backend/cancellation errors remain typed until the presentation boundary.
- [ ] Run:

```powershell
cargo test -p opi-coding-agent --test interactive_auth
cargo test -p opi-coding-agent --test oauth_auth login_oauth_
cargo test -p opi-coding-agent --test oauth_auth logout_credential_
cargo test -p opi-coding-agent --lib oauth_login_restores_terminal_after_flow_failure
```

Expected: new dispatcher tests pass; the obsolete inline test is deleted or renamed to the RAII guard test in `interactive_auth.rs`.

### Step 4: Task gates and commit boundary

- [ ] Run `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- [ ] If authorized, stage only Task 2 files and commit:

```powershell
git commit -m "feat(opi-coding-agent): centralize interactive auth commands"
```

---

## Task 3: Implement 14.10 Live Auth and Session Interaction

**Files:**

- Modify: `crates/opi-coding-agent/src/credential_store.rs`
- Modify: `crates/opi-coding-agent/src/provider_factory.rs`
- Modify: `crates/opi-coding-agent/src/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/src/interactive.rs`
- Modify: `crates/opi-coding-agent/src/runner.rs`
- Modify: `crates/opi-coding-agent/src/rpc.rs`
- Modify: `crates/opi-coding-agent/tests/provider_factory.rs`
- Modify: `crates/opi-coding-agent/tests/oauth_auth.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/tests/json_mode.rs`
- Modify: `crates/opi-coding-agent/tests/non_interactive.rs`
- Modify: `crates/opi-coding-agent/tests/rpc_jsonl.rs`

### Step 1: Add failing lazy-auth, refresh-timeout, and mode tests

- [ ] Add four factory-built `wiremock` tests in `oauth_auth.rs`: `factory_built_approved_profiles_resolve_auth_inside_each_stream`, `factory_stream_reresolves_after_store_change`, `factory_built_approved_profiles_map_revocation_without_retry`, and `refresh_timeout_releases_lock_and_preserves_prior_credential`.

The first constructs Anthropic/Copilot/Codex with no credential, then polls one stream and expects `CredentialNeeded { provider_id }` before `wiremock` records a request. The second streams twice around a fake-store token replacement and asserts two different Authorization values. The third returns auth-invalid for each profile and asserts one request, `CredentialRevoked`, and `is_retryable() == false`. The fourth uses a pending refresh provider and a 20 ms resolver timeout, then proves another mutation acquires the lock and the old credential is unchanged.

- [ ] Add `interactive_explicit_login_retries_pending_turn_once` to `interactive_auth.rs`: a first run yields pre-output `CredentialNeeded`, the UI records only remediation, the user dispatches `/login anthropic`, and the pending prompt is retried once without appending a second user message.
- [ ] Add `json_mode_credential_needed_emits_typed_remediation_without_prompt` to `json_mode.rs`. Assert schema version, provider id `anthropic`, remediation `/login anthropic`, auth-failure exit code, and completion within a short timeout.
- [ ] Add matching RPC/text assertions that no presenter starts and no stdin read occurs.
- [ ] Run these tests. Expected red: Anthropic fails at construction, env OAuth reports the env-var name, refresh never times out, and interactive mode auto-starts login.

### Step 2: Add lazy API-key and correct env-OAuth auth sources

- [ ] Extend `AuthSource`:

```rust
pub enum AuthSource {
    Baked(SecretString),
    ApiKey {
        resolver: Arc<CredentialResolver>,
        provider_id: String,
        env_var: String,
    },
    Store {
        resolver: Arc<CredentialResolver>,
        provider_id: String,
        oauth: Arc<dyn OAuthProvider>,
    },
    EnvOAuthToken {
        provider_id: String,
        env_var: String,
        env_lookup: EnvLookup,
    },
}
```

- [ ] Implement `ApiKey::resolve` by awaiting the new `Result<Option<ResolvedApiKey>, CredentialStoreError>` and returning `CredentialNeeded { provider_id }` on `Ok(None)`. Convert store corruption/backend failures to redacted, non-retryable `ProviderError::Config`.
- [ ] Implement `EnvOAuthToken` missing-value errors with its `provider_id`, never `env_var`.
- [ ] Change non-OAuth API-key factory paths that support lazy auth to `AuthSource::ApiKey`; do not call `resolve_api_key` during construction.
- [ ] Keep Copilot/Codex construction lazy through `AuthSource::Store`. Remove the Anthropic `has_oauth_credential` eager routing decision: select one composite Anthropic resolver with precedence `stored OAuth -> ANTHROPIC_OAUTH_TOKEN -> stored API key -> ANTHROPIC_API_KEY` at each stream poll.
- [ ] Keep OAuth scope limited to Anthropic, GitHub Copilot, and OpenAI Codex. Do not add another OAuth provider while changing live resolution.

Use one explicit variant rather than factory-time branching:

```rust
Anthropic {
    resolver: Arc<CredentialResolver>,
    oauth: Arc<dyn OAuthProvider>,
    oauth_env_var: String,
    api_key_env_var: String,
}
```

Its resolver checks stored OAuth, then the OAuth env token, then API-key resolution, and reports `CredentialNeeded { provider_id: "anthropic" }` only after all sources are absent.

Implement the precedence from one typed store read: a stored OAuth envelope calls `resolve_oauth`; otherwise a non-empty OAuth env token wins; otherwise a stored API-key envelope wins; otherwise the API-key env is used. On `BackendUnavailable`, try the two environment sources in that order. Any malformed/unknown/other backend error returns immediately. This avoids treating a stored API-key envelope as stored OAuth merely because `probe()` reported `Present`.

### Step 3: Bound refresh while holding the mutation lock

- [ ] Add `refresh_timeout: Duration` to `CredentialResolver`, defaulting to 30 seconds, and a test-only `with_refresh_timeout` builder.
- [ ] Wrap only the refresh HTTP future while the lock guard is live:

```rust
let refreshed = tokio::time::timeout(self.refresh_timeout, oauth.refresh(&cred)).await;
match refreshed {
    Ok(Ok(refreshed)) => {
        self.store
            .write_unlocked(provider_id, &Credential::from(refreshed.clone()))
            .await
            .map_err(store_err_to_provider)?;
        Ok(ResolvedAuth { scheme: AuthScheme::Bearer, secret: refreshed.access })
    }
    Ok(Err(refresh_error)) => self.resolve_after_failed_refresh(provider_id, refresh_error).await,
    Err(_) => Err(ProviderError::AuthFailed(format!(
        "OAuth refresh timed out for provider '{provider_id}'"
    ))),
}
```

The timeout error is `AuthFailed`, not `ProviderError::Timeout`, because it must be non-retryable. Do not write on timeout.

### Step 4: Replace auto-login with explicit pending-turn retry

- [ ] Track `pending_credential_provider: Option<String>` beside interactive state.
- [ ] On pre-output `AgentError::CredentialNeeded`, set the pending provider, show `credential needed for '<id>'; run /login <id>`, return to idle, and do not call the presenter.
- [ ] When `dispatch_auth_command` returns `LoggedIn { provider_id }`, retry only if it matches the pending provider. Call `retry_last_prompt()` once, then clear the pending provider before spawning the retry so repeated success events cannot retry twice.
- [ ] Do not append a user message for retry. A new ordinary prompt clears any stale pending provider. Failed/cancelled login does not retry.
- [ ] Keep JSON/RPC/text formatting typed and non-blocking. All modes use the `ProviderError`/`AgentError` provider id and `/login <provider>` remediation; none invokes the auth dispatcher.

### Step 5: Run focused live-path tests

- [ ] Run:

```powershell
cargo test -p opi-coding-agent --test provider_factory build_provider_returns_auth_error_for_missing_credentials
cargo test -p opi-coding-agent --test oauth_auth factory_built_approved_profiles_resolve_auth_inside_each_stream
cargo test -p opi-coding-agent --test oauth_auth factory_stream_reresolves_after_store_change
cargo test -p opi-coding-agent --test oauth_auth factory_built_approved_profiles_map_revocation_without_retry
cargo test -p opi-coding-agent --test oauth_auth refresh_timeout_releases_lock_and_preserves_prior_credential
cargo test -p opi-coding-agent --test interactive_auth interactive_explicit_login_retries_pending_turn_once
cargo test -p opi-coding-agent --test json_mode json_mode_credential_needed_emits_typed_remediation_without_prompt
cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt
cargo test -p opi-coding-agent --test oauth_auth rpc_credential_needed_fails_without_blocking
```

Update the provider-factory test's expectation: provider construction succeeds without credentials; the first polled stream produces typed `CredentialNeeded`.

### Step 6: Task gates and commit boundary

- [ ] Run `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- [ ] If authorized, stage only Task 3 files and commit:

```powershell
git commit -m "fix(opi-coding-agent): resolve auth inside live streams"
```

---

## Task 4: Implement 14.11 Factory-Built Anthropic Cache Markers

**Files:**

- Modify: `crates/opi-coding-agent/src/provider_factory.rs` only if a test-only shared-client injection seam is required
- Create: `crates/opi-coding-agent/tests/anthropic_cache_markers.rs`

### Step 1: Add the failing production-path test

- [ ] Create one table-driven integration test, `factory_built_anthropic_cache_markers_follow_final_capabilities`, with four cases:

```rust
struct Case {
    model: &'static str,
    retention: CacheRetention,
    expect_markers: bool,
    expect_ttl: Option<&'static str>,
}

let cases = [
    Case { model: "claude-sonnet-4-5-20250514", retention: CacheRetention::Long, expect_markers: true, expect_ttl: Some("1h") },
    Case { model: "claude-sonnet-4-5-20250514", retention: CacheRetention::Short, expect_markers: true, expect_ttl: None },
    Case { model: "claude-sonnet-4-5-20250514", retention: CacheRetention::Disabled, expect_markers: false, expect_ttl: None },
    Case { model: "custom-anthropic-model", retention: CacheRetention::Long, expect_markers: false, expect_ttl: None },
];
```

For each case: build the provider through the real coding-agent factory, inject fake auth and `wiremock`, select the final `ModelInfo`, stream a request containing a system prompt, user text, assistant text, and two tools, then inspect the captured JSON body. Assert markers only on system, final user text, final assistant text, and final tool definition, with exact TTL semantics.

- [ ] Run the test. Expected red if the factory/capability/stream path does not expose test HTTP injection or if any final capability is lost.

### Step 2: Add only the minimum factory seam

- [ ] If the existing config base URL and provider shared-client APIs are sufficient, make no production change. Otherwise add an internal/test-visible factory input:

```rust
pub struct ProviderFactoryOverrides {
    pub http_client: Option<Arc<opi_ai::http::HttpClient>>,
}
```

Production passes `ProviderFactoryOverrides::default()`; tests pass the `wiremock` client. Do not add a public CLI/config option.
- [ ] Preserve existing private helper tests; do not count them as the owning acceptance scenario.
- [ ] Run:

```powershell
cargo test -p opi-coding-agent --test anthropic_cache_markers factory_built_anthropic_cache_markers_follow_final_capabilities
cargo test -p opi-ai --lib anthropic::tests::cache_control_long_ttl_emits_all_markers
cargo test -p opi-ai --test model_capabilities_migration anthropic_builtin_models_have_cache_capabilities
```

### Step 3: Task gates and commit boundary

- [ ] Run `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- [ ] If authorized, stage only the test and any minimum factory seam, then commit:

```powershell
git commit -m "test(opi-coding-agent): cover factory cache markers"
```

---

## Task 5: Implement 14.12 Usage and Cost Contract

**Files:**

- Modify: `crates/opi-ai/src/stream.rs`
- Modify: `crates/opi-ai/src/anthropic.rs`
- Modify: `crates/opi-ai/src/openai_chat.rs`
- Modify: `crates/opi-ai/src/openai_responses.rs`
- Modify: `crates/opi-ai/src/bedrock/mod.rs`
- Modify: `crates/opi-ai/src/gemini.rs`
- Modify: `crates/opi-ai/tests/usage_cost.rs`
- Modify: `crates/opi-ai/tests/anthropic_fixtures.rs`
- Modify: `crates/opi-ai/tests/openai_chat_fixtures.rs`
- Modify: `crates/opi-ai/tests/openai_responses_fixtures.rs`
- Modify: `crates/opi-coding-agent/src/harness.rs`
- Modify: `crates/opi-coding-agent/src/session_coordinator.rs`
- Modify: `crates/opi-coding-agent/tests/session_runtime.rs`
- Modify: `CHANGELOG.md`

### Step 1: Pin the public contract with failing tests

- [ ] Replace the current zero/default assertions in `usage_cost.rs` with compile/runtime assertions for:

```rust
let usage = Usage {
    input_tokens: 10,
    output_tokens: 20,
    cache_read_tokens: 3,
    cache_write_tokens: 4,
    cache_write_1h_tokens: Some(2_u64),
    reasoning_tokens: Some(5_u64),
    reported: true,
};
let _: Option<u64> = usage.cache_write_1h_tokens;
let _: Option<u64> = usage.reasoning_tokens;
```

- [ ] Add serde cases proving missing -> `None`, explicit zero -> `Some(0)`, equality to parent -> valid, and nonzero -> preserved.
- [ ] Remove all references to `CostBreakdown::cache_write_1h_cost` from expected literals. Add a compile-fail rustdoc example or source guard proving only `input_cost`, `output_cost`, `cache_read_cost`, and `cache_write_cost` remain.
- [ ] Change malformed fixture tests to expect one `ProviderError::StreamError`, no terminal `Done` with invalid usage, and `is_retryable() == false`.
- [ ] Add `phase14_usage_subsets_survive_session_resume` in `session_runtime.rs` with nonzero 1-hour/reasoning values and before/after cumulative/cost equality.
- [ ] Run the focused tests. Expected red from type mismatches, clamping, the extra cost line, and lost resume values.

### Step 2: Correct `Usage`, cumulative aggregation, and cost

- [ ] Change the two child fields and constructor:

```rust
pub cache_write_1h_tokens: Option<u64>,
pub reasoning_tokens: Option<u64>,

pub fn reported(
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
    cache_write_1h_tokens: Option<u64>,
    reasoning_tokens: Option<u64>,
) -> Self
```

`Usage::unknown()` sets both to `None`. Parent counters remain `u32` as reviewed.

- [ ] Update Bedrock and Gemini `Usage::reported` call sites to pass `None` for child fields their upstream responses do not report. Do not treat absence as `Some(0)`.

- [ ] Change `CumulativeUsage` child totals, `from_totals` inputs, and accessors to `Option<u64>`. Accumulate with this exact missing-vs-reported rule:

```rust
fn add_optional(total: &mut Option<u64>, value: Option<u64>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0).saturating_add(value));
    }
}
```

- [ ] Remove `cache_write_1h_cost`. Calculate the weighted parent line once:

```rust
let one_hour = usage.cache_write_1h_tokens.unwrap_or(0);
let short = (usage.cache_write_tokens as u64).saturating_sub(one_hour);
CostBreakdown {
    input_cost: usage.input_tokens as f64 * per_tok(pricing.input_cost_per_mtok),
    output_cost: usage.output_tokens as f64 * per_tok(pricing.output_cost_per_mtok),
    cache_read_cost: usage.cache_read_tokens as f64 * per_tok(pricing.cache_read_cost_per_mtok),
    cache_write_cost: short as f64 * per_tok(pricing.cache_write_cost_per_mtok)
        + one_hour as f64 * per_tok(pricing.input_cost_per_mtok) * 2.0,
}
```

`total_cost()` sums the four parent lines once. Reasoning remains inside `output_cost`.

### Step 3: Reject malformed provider subsets

- [ ] Add one internal validator used by all three mappers:

```rust
pub(crate) fn validate_usage_subsets(usage: &Usage) -> Result<(), ProviderError> {
    if usage
        .cache_write_1h_tokens
        .is_some_and(|child| child > u64::from(usage.cache_write_tokens))
    {
        return Err(ProviderError::StreamError(
            "cache_write_1h_tokens exceeds cache_write_tokens".to_owned(),
        ));
    }
    if usage
        .reasoning_tokens
        .is_some_and(|child| child > u64::from(usage.output_tokens))
    {
        return Err(ProviderError::StreamError(
            "reasoning_tokens exceeds output_tokens".to_owned(),
        ));
    }
    Ok(())
}
```

- [ ] Parse optional child fields as `Option<u64>` in Anthropic, OpenAI Chat, and OpenAI Responses. Remove every `unwrap_or(0)`, clamp, and “unknown usage” fallback used for an invalid subset.
- [ ] Change each mapper stage that can receive usage to return `Result<..., ProviderError>`. Validate before mutating the partial assistant message or emitting a terminal event. On error, emit/return only `StreamError` and stop processing that response.

### Step 4: Preserve optional fields across runtime persistence

- [ ] Update `opi-agent/src/harness.rs` and `session_coordinator.rs` to use `add_optional` semantics when accumulating/replaying assistant usage.
- [ ] Keep the session schema version unchanged. Existing entries missing the two serde fields deserialize to `None`; new entries serialize the optional usage fields on assistant messages but never serialize calculated cost.
- [ ] Make `phase14_usage_subsets_survive_session_resume` write a real session through `SessionCoordinator`, open it with a fresh reader/harness, reconstruct the active branch, and compare:

```rust
assert_eq!(before.total_cache_write_1h_tokens(), Some(7));
assert_eq!(before.total_reasoning_tokens(), Some(11));
assert_eq!(after, before);
assert_eq!(after_cost, before_cost);
```

### Step 5: Record the 0.x break and run focused tests

- [ ] Under `CHANGELOG.md` `## [Unreleased]` / `### Breaking Changes`, state that child usage fields are `Option<u64>`, malformed subsets now error, and `cache_write_1h_cost` was removed in favor of a weighted `cache_write_cost` parent line.
- [ ] Run:

```powershell
cargo test -p opi-ai --test usage_cost
cargo test -p opi-ai --test anthropic_fixtures cache_1h_
cargo test -p opi-ai --test openai_chat_fixtures reasoning_
cargo test -p opi-ai --test openai_responses_fixtures responses_reasoning_
cargo test -p opi-coding-agent --test session_runtime phase14_usage_subsets_survive_session_resume
```

Expected: all valid absence/zero/equality/nonzero cases pass; each malformed case returns nonretryable `StreamError`; session resume preserves exact optional totals and cost.

### Step 6: Task gates and commit boundary

- [ ] Run `cargo fmt --check --all`, `cargo clippy --workspace --all-targets -- -D warnings`, and `git diff --check`.
- [ ] If authorized, stage only Task 5 files and commit:

```powershell
git commit -m "fix(opi-ai): restore usage and cost contract"
```

---

## Task 6: Implement 14.13 Final Documentation, Runtime Evidence, and Exit Closure

**Files:**

- Modify: `docs/opi-spec.md`
- Modify: `docs/opi-spec.zh.md`
- Modify: `README.md`
- Modify: `README.zh.md`
- Modify: `crates/opi-ai/README.md`
- Modify: `crates/opi-ai/README.zh.md`
- Modify: any other root/crate README pair that contains a Phase 14 claim found by the audit search
- Modify: `crates/opi-ai/src/credential.rs`
- Modify: `crates/opi-ai/src/provider.rs`
- Modify: `crates/opi-ai/src/stream.rs`
- Modify: `crates/opi-coding-agent/src/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/src/runner.rs`
- Modify: `crates/opi-coding-agent/src/rpc.rs`
- Modify: `crates/opi-coding-agent/tests/phase14_provider_auth_docs.rs`
- Modify: `crates/opi-coding-agent/tests/interactive_auth.rs`
- Modify: `crates/opi-coding-agent/tests/json_mode.rs`
- Modify: `CHANGELOG.md`
- Tool-owned update: `.opi-impl-state.json` through `opi-implement` only
- Create/update artifacts: `target/opi-artifacts/phase14-task14.13/`

### Step 1: Add failing truthfulness and runtime-help guards

- [ ] Update `phase14_provider_auth_docs.rs` assertions before editing docs. Pin:
  - native store names and all six release targets;
  - explicit `/login` behavior and no automatic login after `CredentialNeeded`;
  - optional `u64` usage children and the four-line `CostBreakdown`;
  - Anthropic cache markers are emitted for capable models;
  - removal of `~/.local/share/opi/auth/` from the source/runtime layout;
  - exact corrected Request/Provider snippets;
  - `api-map` disposition and new trigger;
  - final status `implemented; remediation complete` once Tasks 14.8-14.13 pass their owned gates.
- [ ] Add `interactive_auth_help_is_discoverable_through_dispatcher`: render `/help` from the same command registry used by production and assert both auth commands and descriptions.
- [ ] Keep the Task 3 JSON behavior test as owning runtime evidence. Add RPC/text negative presenter assertions if they are not already in the Task 3 tests.
- [ ] Run the three guards. Expected red because docs/help still contain the audited stale claims.

### Step 2: Align English/Chinese docs, rustdoc, help, and changelog

- [ ] Update English and Chinese files in the same edit. Do not claim dynamic model refresh beyond substrate-only.
- [ ] Replace the opi-ai README sentence denying Anthropic markers with the actual capability/retention behavior.
- [ ] Remove the unused auth-directory line from both spec layouts.
- [ ] Copy the exact public signatures from compiled source into normative snippets:

```rust
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub cache_write_1h_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub reported: bool,
}

pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub cache_read_cost: f64,
    pub cache_write_cost: f64,
}
```

- [ ] Record `api-map` verbatim as `deferred-by-updated-design` with this trigger: one catalog/provider identity must require at least two concrete wire families and explicit provider profiles must be inadequate; a separate reviewed design must then define model-to-wire selection, per-stream auth, capability routing, and the `ProviderCollection` boundary.
- [ ] Ensure `/help`, JSON, RPC, and text remediation all use `provider_id` and `/login <provider>`; never mention an env-var name as the provider.
- [ ] Change both Phase 14 status lines from the Task 0 interim state to `implemented; remediation complete` / `已实现；修复已完成` only after every owned Task 14.8-14.13 focused gate is green. Archive eligibility remains a separate phase-exit decision.
- [ ] Rerun the docs/help/JSON tests. Expected green.

### Step 3: Rerun every repaired and corrective acceptance command

- [ ] Run every Appendix A command and record target, filter, selected count, exit code, and result under `target/opi-artifacts/phase14-task14.13/PHASE_EXIT_SCENARIO_AUDIT.md`.
- [ ] Run every new 14.8-14.13 acceptance command recorded by reviewed reinit. Zero selected tests is a failure even when Cargo exits zero.
- [ ] Use the `opi-implement` workflow to append evidence and advance statuses; never edit the ledger JSON directly.

### Step 4: Run final workspace gates in order

- [ ] Run:

```powershell
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
$env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --no-deps
Remove-Item Env:RUSTDOCFLAGS
cargo test --workspace --doc
cargo test --workspace --all-targets
& 'scripts\opi-impl-smoke.ps1'
```

Expected: every command exits zero. Only the three `*.workflow.js` files use the reviewed relocated path; the smoke scripts remain under root `scripts/`. Do not revive or stage the deleted root `scripts/*.workflow.js` files.

- [ ] Run `git diff --check` and scan changed files/artifacts for secret canaries, absolute temp roots, unfinished-work markers, zero-test output, and stale `cache_write_1h_cost` references.
- [ ] Run the Phase 14 artifact audit against `target/opi-artifacts/phase14-task14.13/`; expected issue count is zero.

### Step 5: Commit the pre-exit readiness task

- [ ] If authorized, stage only Task 6 docs/tests/help/ledger-owned files and commit:

```powershell
git commit -m "docs(phase14): close provider auth exit gaps"
```

- [ ] Mark 14.13 passing only after its owned focused tests, all corrective scenarios, the 29-command selected-count audit, full workspace gates, and artifact audit pass. Phase F is downstream and is not part of 14.13's own pass condition.

### Step 6: Run downstream Phase F after 14.13 passes

- [ ] After Task 6 review is clean and the `opi-implement` ledger records 14.13 as passing, invoke the reviewed Phase F exit process. Run all five lenses independently and adversarially verify every foldable finding.
- [ ] Accept only `met` or the exactly cited `deferred-by-updated-design` `api-map` residual. Any `not-met` leaves Phase 14 active and returns to the owning corrective task.

- [ ] Do not create an archive snapshot or archive commit until the phase-exit report is clean and the user separately accepts the archive gate.

---

## Appendix A: Corrected Historical 29-Command Matrix

These commands repair only the historical 14.1-14.7 metadata. Every command names a current target and selects at least one existing test before corrective implementation. New 14.8-14.13 scenarios own the stronger product-path evidence added by this plan.

Authoring validation on 2026-07-14 ran every command with `-- --list`: commands 1-14 and 16-29 each selected one test; command 15 selected the three approved factory-route tests; all 29 invocations exited zero.

1. `cargo test -p opi-coding-agent --test doctor_cli stored_credential_probe_is_redacted`
2. `cargo test -p opi-coding-agent --test list_models stored_credential_metadata_is_redacted`
3. `cargo test -p opi-coding-agent --test credential_store headless_api_key_env_fallback`
4. `cargo test -p opi-coding-agent --test credential_store keychain_store_reaches_production_construction`
5. `cargo test -p opi-coding-agent --test oauth_auth all_builtin_flows_support_manual_fallback`
6. `cargo test -p opi-coding-agent --test oauth_auth resolve_oauth_near_expiry_refreshes_and_writes_new_token`
7. `cargo test -p opi-coding-agent --lib oauth_login_restores_terminal_after_flow_failure`
8. `cargo test -p opi-coding-agent --test oauth_auth rpc_credential_needed_fails_without_blocking`
9. `cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt`
10. `cargo test -p opi-coding-agent --test oauth_auth anthropic_oauth_revoked_stops_turn_without_retry_or_relogin`
11. `cargo test -p opi-coding-agent --test oauth_auth login_oauth_writes_oauth_credential_to_store`
12. `cargo test -p opi-coding-agent --test oauth_auth anthropic_env_oauth_token_precedence_stored_wins_env_fallback`
13. `cargo test -p opi-coding-agent --test credential_store mutation_lock_serializes_concurrent_writers`
14. `cargo test -p opi-coding-agent --test oauth_auth resolve_oauth_refresh_failure_rereads_fresh_token_without_partial_write`
15. `cargo test -p opi-coding-agent --test oauth_auth factory_routes_`
16. `cargo test -p opi-ai --test request_enrichment request_scalars_carry_explicit_values`
17. `cargo test -p opi-coding-agent --test provider_factory build_provider_wires_each_builtin_provider_family`
18. `cargo test -p opi-agent --test agent_loop_mock session_id_reaches_every_request`
19. `cargo test -p opi-coding-agent --test session_runtime phase14_session_affinity_tracks_new_resume_and_fork`
20. `cargo test -p opi-ai --test request_enrichment session_affinity_wire_mappings`
21. `cargo test -p opi-ai --lib anthropic::tests::cache_control_long_ttl_emits_all_markers`
22. `cargo test -p opi-ai --test model_capabilities_migration anthropic_builtin_models_have_cache_capabilities`
23. `cargo test -p opi-ai --test anthropic_fixtures cache_1h_tokens_parsed_as_subset_of_cache_write`
24. `cargo test -p opi-coding-agent --test session_runtime open_existing_replays_usage_from_assistant_messages`
25. `cargo test -p opi-ai --test usage_cost calculate_cost_all_cache_writes_are_1h`
26. `cargo test -p opi-ai --test provider_collection refresh_models_is_atomic_substrate`
27. `cargo test -p opi-coding-agent --test phase14_provider_auth_docs localized_docs_pin_exact_phase14_claims_and_acceptance_rows`
28. `cargo test -p opi-coding-agent --test oauth_auth login_logout_commands_are_discoverable`
29. `cargo test -p opi-coding-agent --test non_interactive credential_needed_fails_without_prompt`

## Appendix B: Corrective Acceptance Ownership

| Scenario | Owning task | Exact focused command |
|---|---:|---|
| Native target selection and guard | 14.8 | `cargo test -p opi-coding-agent --lib native_keyring_host_selection_installs_a_default_store` |
| Credential-aware production startup ordering | 14.8 | the four exact `native_keyring_precedes_*` commands in Task 1 Step 5 |
| Corrupt/unknown envelope does not fall through | 14.8 | `cargo test -p opi-coding-agent --test credential_store resolver_` |
| Async doctor probe | 14.8 | `cargo test -p opi-coding-agent --test doctor_cli doctor_store_probe_uses_async_command_orchestration` |
| Stored-only listing | 14.8 | `cargo test -p opi-coding-agent --test list_models stored_only_credential_lists_models_through_async_orchestration` |
| Production auth dispatcher | 14.9 | `cargo test -p opi-coding-agent --test interactive_auth interactive_auth_dispatcher_` |
| Lazy/per-stream auth | 14.10 | `cargo test -p opi-coding-agent --test oauth_auth factory_built_approved_profiles_resolve_auth_inside_each_stream` |
| Changed-store stream auth | 14.10 | `cargo test -p opi-coding-agent --test oauth_auth factory_stream_reresolves_after_store_change` |
| Refresh timeout and lock release | 14.10 | `cargo test -p opi-coding-agent --test oauth_auth refresh_timeout_releases_lock_and_preserves_prior_credential` |
| Same-turn explicit login retry | 14.10 | `cargo test -p opi-coding-agent --test interactive_auth interactive_explicit_login_retries_pending_turn_once` |
| Typed JSON remediation | 14.10 | `cargo test -p opi-coding-agent --test json_mode json_mode_credential_needed_emits_typed_remediation_without_prompt` |
| All-profile revocation | 14.10 | `cargo test -p opi-coding-agent --test oauth_auth factory_built_approved_profiles_map_revocation_without_retry` |
| Factory cache-marker wire path | 14.11 | `cargo test -p opi-coding-agent --test anthropic_cache_markers factory_built_anthropic_cache_markers_follow_final_capabilities` |
| Usage/cost public contract | 14.12 | `cargo test -p opi-ai --test usage_cost` |
| Malformed provider subsets | 14.12 | `cargo test -p opi-ai --test anthropic_fixtures cache_1h_` plus both OpenAI reasoning filters |
| Nonzero session resume | 14.12 | `cargo test -p opi-coding-agent --test session_runtime phase14_usage_subsets_survive_session_resume` |
| Runtime auth help | 14.13 | `cargo test -p opi-coding-agent --test interactive_auth interactive_auth_help_is_discoverable_through_dispatcher` |
| Localized truth/Non-Goals/api-map | 14.13 | `cargo test -p opi-coding-agent --test phase14_provider_auth_docs` |

## Self-Review Checklist

- [ ] Coverage: B01-B04 -> 14.8; B06-B07 -> 14.9; B08-B11 -> 14.10; B12 -> 14.11; B14-B17 -> 14.12; B05/B13/B18-B20 -> 14.13.
- [ ] Type consistency: child usage fields are `Option<u64>` everywhere; cumulative child accessors are optional; parent token buckets remain `u32`; cost has four lines.
- [ ] Error consistency: absence -> `CredentialNeeded`; platform unavailability is distinct; corrupt/unknown envelopes propagate; refresh timeout is non-retryable `AuthFailed`; malformed usage is non-retryable `StreamError`.
- [ ] Mode consistency: only explicit TUI `/login` invokes a presenter; JSON/RPC/text never prompt or auto-login.
- [ ] Security consistency: no real keychain/network/browser/terminal/user session in tests; no secret appears in diagnostics, artifacts, or model listing.
- [ ] Residual consistency: `api-map` is reassessed and deferred with a new product trigger; no Phase 14 implementation is added.
- [ ] Unfinished-marker scan: construct the search terms from split strings so the plan does not match its own check, then require no matches:

```powershell
$needles = @(('TO' + 'DO'), ('TB' + 'D'), ('place' + 'holder'), ('fill' + ' this'), ('implement' + ' later'))
rg -n ($needles -join '|') docs/superpowers/plans/2026-07-14-phase14-exit-remediation.md
```
- [ ] Baseline preservation: workflow relocation paths remain unstaged and outside all corrective commits.
