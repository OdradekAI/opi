# Batch EXT/PKG — extensions, packages, model config, MCP, diagnostics

Batch: EXT/PKG (dims 7,12,3,15,17), measured 2026-11-06 against pi 1.5.0 (8aefc4c) and opi 0.8.3 (d382688, clean).
Contracts measured: 38 (27 EXT + 11 PKG). Verdicts: equivalent 0 · partial 13 · missing 16 · different 5 · opi-only 4.
Biggest gaps: the entire extension JS API surface (UI context, ~49 event types, codemode/tool-search built-ins), full MCP product integration, virtual models, remote catalog refresh, package update/self-update.
Uncertainties: dimension-number mapping taken from the brief's title order (ext=7, pkg=12, model-config=3, mcp=15, diag=17); pi runner.ts and extensions/mcp/index.ts read via module docs + selected sections, not line-by-line.

All records: evidence basis = source inspection; review outcome = self-reviewed.

---

## EXT — extension model, built-in extensions, model config, MCP, diagnostics

### EXT-01 — extension runtime model (in-process TS vs process adapter)
- Dimension(s): 7
- pi entry: `packages/coding-agent/src/core/extensions/loader.ts:880` (discoverAndLoadExtensions: `.pi/extensions` project + `~/.pi/agent/extensions` global, one level deep, jiti-loaded TS/JS); `core/extensions/jiti-loader.ts:1`; `core/extensions/types.ts:1616` (`ExtensionAPI` handed to a JS factory in the pi process)
- Opi entry: `crates/opi-agent/src/extension.rs:175` (`Extension` trait); `crates/opi-coding-agent/src/adapter_extension.rs:783` (`start_adapters_from_packages` spawns `[adapter]` child processes); `crates/opi-coding-agent/src/adapter_protocol.rs:86` (JSONL `opi-extension-jsonl-v1`); `crates/opi-coding-agent/src/resource.rs:72` (an "extension" resource is only a `name/version/description` TOML manifest — no code)
- Comparison: different
- Observable difference: a pi extension is TypeScript running in-process with full access to `pi` APIs and the session; an opi extension is a directory with `extension.toml` metadata plus an optional out-of-process JSONL adapter. Opi extension directories without `[adapter]` contribute metadata only (`harness.rs:829` "Discovered extensions" prompt section). pi auto-discovers loose `.ts`/`.js` files and `index.ts` packages; opi requires `extension.toml` per extension dir. pi supports `-e path`, `builtin:<name>` (loader.ts:25/1049 area, `packages.md`), `/reload` hot reload, hidden/replaceable inline extensions; opi has no reload, no `-e`, no builtin-extension mechanism (absence: `grep -rn 'reload' crates/opi-coding-agent/src` → no runtime reload hits).
- Notes: process isolation is the deliberate opi design; under the strict reproduction lens it is still `different`.

### EXT-02 — extension event/hook surface
- Dimension(s): 7
- pi entry: `packages/coding-agent/src/core/extensions/types.ts:1425` (`ExtensionEvent` union, ~49 event types incl. `project_trust`, `session_*` (10), `context`, `context_with_system`, `before/after_provider_request/headers/response`, `provider_stream_event`, `cache_warming_decision`, `before_agent_start`, `agent_start/end/before_settle/settled`, `turn_start/end`, `message_*`, `tool_execution_*`, `model_select`, `thinking_level_select`, `user_bash`, `input`, `tool_call`, `tool_result`, `ui_prompt_*`, `mcp_servers_change`); docs `docs/extensions.md:307-1160` (ordering guarantees)
- Opi entry: `crates/opi-agent/src/extension.rs:214-260` (`on_before_tool_call`, `on_after_tool_call` (observer-only), `transform_context`, `prepare_next_turn`, `on_event(&AgentEvent)`); adapter hook whitelist `adapter_extension.rs:60-100` (`before_tool_call`, `after_tool_call`, `transform_context`, `prepare_next_turn`, `event`)
- Comparison: partial
- Observable difference: opi covers the tool-gate (block, no arg mutation), context transform, next-turn replacement, and generic agent-event observation. Missing: session lifecycle events (start/shutdown/fork/tree/compact), input/user_bash interception, message-stream events, provider request/headers/response/stream hooks at the extension surface (opi-ai has `callbacks.rs:5-28` `transform_headers`/`on_payload`/`on_response`/`on_raw` as a library seam, but only a harness test uses it — `harness.rs:6134` — no extension wiring), settle/turn boundaries with entry drafts, model/thinking selection events, `terminate` on block.
- Notes: pi `tool_call` mutates `event.input` in place; opi `on_before_tool_call` receives `&Value` and cannot mutate. pi `after`-style `tool_result` can replace results; opi `on_after_tool_call` is explicitly observer-only (`extension.rs:227`).

### EXT-03 — extension UI context and custom rendering
- Dimension(s): 7
- pi entry: `packages/coding-agent/src/core/extensions/types.ts:164` (`ExtensionUIContext`: select/confirm/input/notify, setStatus, widgets above/below editor, setFooter/setHeader, custom components with overlays, editor replacement, paste/set/getEditorText, autocomplete stacking, theme get/set, markdown transformers `registerMarkdownTransformer`, message/entry renderers, tool renderers incl. for unregistered tools)
- Opi entry: absence — `grep -rn 'ui\.\|widget\|overlay\|renderer' crates/opi-agent/src/extension.rs` → no UI concept in the trait; adapter protocol (`adapter_protocol.rs`) has no UI messages
- Comparison: missing
- Observable difference: opi extensions cannot render anything, prompt the user, set status, or replace any TUI element; the only extension→user channel is tool output text.

### EXT-04 — extension tool registration
- Dimension(s): 7
- pi entry: `packages/coding-agent/src/core/extensions/types.ts:599` (`ToolDefinition`: label, promptSnippet, promptGuidelines, constrainedSampling, renderShell, prepareArguments, outputSchema, exposure `direct|model-only|codemode|deferred|hidden`, namespace, reachesTools, annotations, defaultActive, prepareLoadout, executionMode, renderCall/renderResult); nested `ctx.executeTool` (`types.ts:397-420`)
- Opi entry: `crates/opi-coding-agent/src/adapter_protocol.rs:111` (`AdapterToolCapability { name, description, input_schema }` only); `adapter_extension.rs:78` (`ProcessAdapterTool`: definition = name+description+schema; execute via JSONL, text content only (`adapter_extension.rs:143-158` filters to text), execution mode hardcoded Sequential `adapter_extension.rs:216`)
- Comparison: partial
- Observable difference: opi adapter tools are model-declared with a bare schema; no exposure modes, no namespaces/annotations, no prompt guidelines, no output schemas, no custom rendering, no nested tool-calling from one tool (no `executeTool` analog), no dynamic tool loading. Cancellation is bridged (`tokio::select!` + best-effort cancel, `adapter_extension.rs:122-131`).

### EXT-05 — extension commands, flags, shortcuts
- Dimension(s): 7
- pi entry: `core/extensions/types.ts:1616+` (`registerCommand`, `registerShortcut`, `registerFlag`, `getFlag`, `getCommands`; flags feed CLI parsing; shortcuts bind KeyIds); slash-command surface `core/slash-commands.ts`
- Opi entry: `crates/opi-agent/src/extension.rs:135` (`ExtensionCommand { name, id, args }`); `crates/opi-coding-agent/src/adapter_protocol.rs:124` (`AdapterCommandCapability { name, description }`); dispatch `harness.rs:3666` (`dispatch_extension_command`) → `ExtensionRegistry::dispatch_command` (`extension.rs:462`)
- Comparison: partial
- Observable difference: opi supports name+args commands with a JSON result and id correlation. No CLI flags contributed by extensions, no keyboard shortcuts, no argument-completion hooks, no listing surface equivalent to `getCommands()` exposed to extensions.

### EXT-06 — extension state persistence
- Dimension(s): 7
- pi entry: `core/extensions/types.ts` `appendEntry(customType, data)` (custom session entries, visible in transcript/renderable) + session-manager-backed restore; runner invalidate/stale semantics (`core/extensions/loader.ts:190-215`)
- Opi entry: `crates/opi-agent/src/extension.rs:277-303` (`serialize_state`/`restore_state` + async variants); registry map keyed by extension name (`extension.rs:476-507`); harness persists/restores around commands (`harness.rs:3666-3706`, `session_coordinator::latest_extension_state`)
- Comparison: partial
- Observable difference: both survive session restarts, but opi state is one opaque JSON value per extension with no transcript visibility or per-entry typing; pi appends typed custom entries that appear in the session file and can be rendered.

### EXT-07 — extension provider registration
- Dimension(s): 3, 7
- pi entry: `core/extensions/types.ts:1882-1920` (`registerProvider`/`unregisterProvider` with native `Provider` objects or `ProviderConfig` incl. `oauth`, `streamSimple`, `images`, `classifiers`, `refreshModels`; immediate post-bind effect); `core/provider-composer.ts:534` (`validateExtensionProvider`), `:547` (`composeModelProvider`)
- Opi entry: `crates/opi-agent/src/extension.rs:197` (`providers(): Vec<Box<dyn Provider>>`) and `:206` (`model_overrides`); wiring `crates/opi-coding-agent/src/provider_factory.rs:166` (`build_harness_collection` registers extension providers + overrides) and `:216` (`materialize_active_overrides`); adapter path: `adapter_extension.rs:298-315` maps `AdapterModelOverride.model` as both provider id and model id with hardcoded `WireApi::OpenAiCompletions` and 128k/16k caps (documented as a Phase 5 MVP simplification at `adapter_extension.rs:293-296`)
- Comparison: partial
- Observable difference: in-process Rust extensions can contribute providers; process adapters can only contribute model overrides with a lossy mapping. No extension-supplied OAuth, custom streaming, image/classifier APIs, or `refreshModels`.

### EXT-08 — virtual models
- Dimension(s): 3
- pi entry: `core/virtual-models.ts:29` (`VIRTUAL_MODEL_API = "pi-virtual"`), `:170` (`createVirtualModel`), `:201` (`withVirtualModels`); routing request/reason/state-entry contract `virtual-models.ts:50-100`; `registerVirtualModel` in `types.ts:1946`; docs `docs/virtual-models.md`
- Opi entry: absence — `grep -rni 'virtual' crates/opi-ai/src/provider.rs crates/opi-coding-agent/src --include='*.rs' -l` → only `tool/bash.rs` (virtualenv strings)
- Comparison: missing
- Observable difference: no selectable catalog entry can route per-request to a physical model; no router state entries.

### EXT-09 — MCP server registration API (extension-declared servers)
- Dimension(s): 7, 15
- pi entry: `core/extensions/types.ts:1918-1926` (`registerMcpServer`/`unregisterMcpServer`/`getMcpServers`); validation + registry `core/mcp-servers.ts:260` (`validateMcpServerConfig`), `:328` (`McpServerRegistry`); `mcp_servers_change` event `types.ts:771`
- Opi entry: absence — see EXT-25 (no MCP anywhere in opi-coding-agent)
- Comparison: missing

### EXT-10 — extension event bus
- Dimension(s): 7
- pi entry: `core/extensions/types.ts` `events: EventBus` (`pi.events.emit/on(channel, data)`); `core/event-bus.ts`
- Opi entry: absence — no channel pub/sub in `crates/opi-agent/src/extension.rs` or adapter protocol
- Comparison: missing
- Observable difference: opi extensions cannot communicate with each other through named channels.

### EXT-11 — built-in codemode extension
- Dimension(s): 7
- pi entry: `src/extensions/codemode/tool.ts:39` (`codemode` tool: model-authored JS calling other tools through the same pipeline, `store/load` cross-call state persisted as `codemode-store` entries, model catalog access, lazy worker execution); docs `docs/codemode.md`
- Opi entry: absence — `grep -rni 'codemode' crates/*/src --include='*.rs'` → no hits
- Comparison: missing
- Observable difference: no model-facing code-execution-over-tools capability exists in opi.

### EXT-12 — built-in tool-search extension
- Dimension(s): 7
- pi entry: `src/extensions/tool-search/tool.ts:20` (`tool_search` BM25 ranker over tool metadata; loads `codemode`/`deferred` tools into the active set; shared with codemode `searchTools()`)
- Opi entry: absence — `grep -rni 'tool_search|tool-search' crates/*/src --include='*.rs'` → no hits
- Comparison: missing
- Observable difference: all opi adapter tools are always declared; there is no deferred-tool discovery mechanism.

### EXT-13 — built-in llama.cpp extension
- Dimension(s): 3, 7
- pi entry: `src/extensions/llama/index.ts:1` (llama.cpp server provider + `/llama` command UI for model pull/load/sleep via HuggingFace, `provider.ts`, `client.ts`, `ui.ts`, `huggingface.ts`); docs `docs/llama-cpp.md`, `docs/nvidia-rtx-llama-cpp.md`
- Opi entry: `crates/opi-ai/src/api/llama.rs:1-41` (`llama-cpp-classify` classifier API: logprob classification over llama-server `/tokenize`,`/apply-template`,`/completion`)
- Comparison: partial
- Observable difference: opi has only the classification half (single-token logprob scoring); no llama.cpp chat provider, no model management UI/commands, no HuggingFace download integration.
- Notes: Layer H — pi docs/llama-cpp.md is a normative usage doc for an implemented extension; opi's classifier adapter is implemented and marked as the exact pi `llama-cpp-classify` identity (`api/llama.rs:40`).

### EXT-14 — user model configuration file
- Dimension(s): 3
- pi entry: `core/model-config.ts:106-132` (`models.json`: `providers.{id}` with `name/baseUrl/apiKey ($ENV/!cmd)/api/oauth:"radius"/headers/compat/authHeader/models[]/modelOverrides{}`; models support `thinkingLevelMap`, `inputLimits` (image resize/maxPerMessage/maxPerRequest), `cost.tiers`, `promptCache`, `samplingParams(ByThinkingLevel)`, per-model `api`/`baseUrl`/`headers` overrides); schema doc `docs/models-configuration.md` (generated from `ModelsJsonSchemas`); JSONC comments + `$schema` accepted
- Opi entry: `crates/opi-coding-agent/src/config.rs:258` (`ProvidersConfig`: anthropic/openai/openrouter/mistral/openai-responses/google/amazon-bedrock/azure/google-vertex + `openai_compatible{}` profiles + `custom{}`); `config.rs:433` (`OpenAiCompatibleProviderConfig` with `models: Vec<ConfiguredModelConfig>` covering id/display_name/context/max/images/streaming/thinking/instruction_role/max_tokens_field); `config.rs:300` (`CustomProviderConfig` with full `ModelInfo` list incl. thinking maps and pricing)
- Comparison: partial
- Observable difference: opi covers custom + OpenAI-compatible endpoints and per-model metadata, but there is no user-file `modelOverrides` table that patches built-in models field-by-field (overrides only arrive via extensions, EXT-07), no `!command` API-key sourcing at config level (credential resolution is keychain/env), no per-model `samplingParams`, `promptCache`, or `inputLimits` image preprocessing in the user config surface, and no per-model `baseUrl` override. Format JSON vs TOML is not itself a behavioral difference.

### EXT-15 — provider composition/merge semantics
- Dimension(s): 3
- pi entry: `core/provider-composer.ts:547` (`composeModelProvider`: models.json + extension registrations layered over built-ins; `baseUrl`-only override for existing providers; models with matching id replace; compat merged field-wise incl. nested routing tables `provider-composer.ts:113-140`); `core/model-runtime.ts:174` (`ModelRuntime` snapshot, availability, credential ops, `CredentialSynchronizationError:138`)
- Opi entry: `crates/opi-coding-agent/src/provider_factory.rs:1281` (`build_provider`), `:166` (`build_harness_collection`: extension providers + non-active overrides into a `ProviderRegistry`; active-provider overrides materialized onto the provider `:216`), `:92` (`ProviderBundle` with per-call auth resolvers and eager extra routes for cross-provider switching)
- Comparison: partial
- Observable difference: both compose custom+extension models over built-ins, but opi has no `unregisterProvider`, no runtime re-composition (registry locks after share, `extension.rs:435`), and extension model overrides can fail with a diagnostic when the wire has no concrete route (`provider_factory.rs:240-247`). pi composes per-request auth (`AuthStatus` sources: stored/runtime/environment/fallback/models_json, `provider-composer.ts:112`).

### EXT-16 — remote model catalog refresh
- Dimension(s): 3
- pi entry: `core/remote-catalog-provider.ts:16` (4h refresh interval), `:104` (`withRemoteCatalog` overlay per built-in provider from `https://pi.dev` with ETag/If-None-Match + Last-Modified vs generated-at, 4s attempt timeout, chat/image/classifier shards), persisted in `models-store.json` (`model-runtime.ts:225`); OpenRouter personalization `remote-catalog-provider.ts:195`; `pi update --models` (`docs/cli.md:278`)
- Opi entry: `crates/opi-ai/src/catalog/mod.rs:1` (offline bundled catalog, "never performs network I/O"); `crates/opi-ai/src/catalog/http_source.rs:1` (`DynamicCatalogSource`/`HttpCatalogSource` exist but "the SDK registers none by default" — and opi-coding-agent registers none: `grep -rn 'HttpCatalogSource|DynamicCatalogSource' crates/opi-coding-agent/src` → no hits); product listing limited to 11 built-in provider ids (`provider_factory.rs:517`) + static offline catalogs `github_copilot.rs:1` / `openai_codex.rs:1` ("pinned to pi 0.80.6")
- Comparison: partial
- Observable difference: opi ships a byte-for-byte pi published catalog artifact (`resources/catalog/builtin/manifest.json`: 42 providers, 1608 chat models) but pinned to pi-ai **1.3.0** (`.repo/pi-1.3.0`), not the target 1.5.0, and `--list-models` surfaces only 11 providers plus configured profiles; there is no automatic or user-triggered catalog refresh, no pi.dev overlay, no OpenRouter key personalization.
- Notes: Layer H — catalog pin provenance is recorded in the artifact manifest; revision drift vs the 1.5.0 target is an observable content difference.

### EXT-17 — Radius gateway provider surface
- Dimension(s): 3
- pi entry: `core/radius.ts:1-11` (`RADIUS_PROVIDER_ID`, `PI_RADIUS_GATEWAY` override, `RADIUS_MCP_URL` gateway MCP endpoint); models.json `oauth: "radius"` (`model-config.ts:100`); bug-report upload to the gateway (`bug-report-upload.ts:26`)
- Opi entry: `"radius"` exists only as a catalog provider id inside the offline artifact (`crates/opi-ai/resources/catalog/builtin/manifest.json` providers list); not in `BUILT_IN_PROVIDER_IDS` (`provider_factory.rs:517`), no dispatch route, no gateway URL env, no MCP endpoint
- Comparison: missing

### EXT-18 — credential storage backend
- Dimension(s): 3
- pi entry: `core/auth-storage.ts:1-60` (`auth.json` in agent dir, proper-lockfile cross-process lock with EPERM retry, 0600 writes, shared read state); `core/runtime-credentials.ts:4` (`RuntimeCredentials` overlay for non-persistent runtime API keys over the CredentialStore)
- Opi entry: `crates/opi-coding-agent/src/credential_store.rs:1-38` (`KeychainCredentialStore` over OS keyring via `native_keyring.rs` guard; `fs4` advisory lock at `<user_config_dir>/credential.lock`; marker+envelope two-step write protocol); runtime overlay: none found (`setRuntimeApiKey` analog absent — `grep -rn 'runtime_api_key|runtimeApiKey' crates` → no hits)
- Comparison: different
- Observable difference: pi persists plaintext JSON on disk with file locks; opi stores secrets in the OS keychain with a marker/envelope protocol and fails closed on wrong-kind reads. Both implement locking; the storage medium, failure modes, and credential visibility differ. Runtime (non-persistent) API key overlay has no opi equivalent.

### EXT-19 — auth guidance messages
- Dimension(s): 3, 17
- pi entry: `core/auth-guidance.ts:8-24` (`formatNoModelsAvailableMessage`, `formatNoModelSelectedMessage`, `formatNoApiKeyFoundMessage` pointing at `providers.md`/`models.md`)
- Opi entry: `crates/opi-coding-agent/src/interactive.rs:1674` (`[credential needed for '{provider_id}': run /login {provider_id}]`), `:1421` (`[model picker: no models available]`), `interactive_auth.rs:166` (`/login` usage)
- Comparison: partial
- Observable difference: opi surfaces equivalent situations inline but has no centralized guidance-message module or docs-path pointers.

### EXT-20 — cache-miss statistics and startup timings
- Dimension(s): 17
- pi entry: `core/cache-stats.ts:1-50` (`CacheMiss`/`CacheWasteTotals`, 5-min TTL heuristic, 1024-token noise floor, cost attribution); `core/timings.ts:1-10` (`PI_TIMING=1` startup profiling with `main`/`extensions` namespaces)
- Opi entry: absence — `grep -rni 'cache_stats|cache_miss|cache waste' crates --include='*.rs'` → no hits; no `OPI_TIMING`-style startup profiler
- Comparison: missing

### EXT-21 — footer git-branch data provider
- Dimension(s): 17
- pi entry: `core/footer-data-provider.ts:11-60` (git metadata walk incl. worktrees and `.git` files, branch via `symbolic-ref`, HEAD file watching with retry)
- Opi entry: absence — no git-branch footer; `crates/opi-tui/src/branch_picker.rs` is the session branch picker, not git; `grep -rn 'symbolic-ref|git branch' crates/opi-coding-agent/src crates/opi-tui/src` → no hits
- Comparison: missing

### EXT-22 — bug report bundle and upload
- Dimension(s): 17
- pi entry: `core/bug-report.ts:17` (`pi.bug-report` custom entry), redaction (`redactUrl`/`redactJsonValue` `bug-report.ts:38-66`), environment/model/provider collection, zip archive; `core/bug-report-upload.ts:12` (multipart upload to Radius `/v1/bug-reports`, optional bearer)
- Opi entry: absence — `grep -rn 'bug_report|bug report' crates` → no hits (opi's `evidence.rs` is a different contract, see EXT-26)
- Comparison: missing

### EXT-23 — crash log
- Dimension(s): 17
- pi entry: `core/crash-log.ts:18` (`crashes.json`, max 5 records, 7-day retention, `uncaught_exception|fatal_error` kinds, extension stack attribution `findExtensionStackMatches`)
- Opi entry: absence — `grep -rn 'crash' crates/opi-coding-agent/src` → only session-export recovery comments (`session_cli.rs:63,652`) and unrelated protocol-host strings; no crash record store
- Comparison: missing

### EXT-24 — install telemetry and client identity
- Dimension(s): 17
- pi entry: `core/telemetry.ts:8` (`PI_TELEMETRY` env override → `enableInstallTelemetry` setting); `core/client-identity.ts:10` (`getClientIdentity`: detailed `User-Agent` with version, `PI_ATTRIBUTION` while telemetry enabled)
- Opi entry: absence — no telemetry opt-in or product client identity; the only User-Agent strings are provider-profile-specific static headers (`github_copilot.rs:12-22`); `crates/opi-agent/src/agent_loop.rs:525` `telemetry_context: None`
- Comparison: missing

### EXT-25 — MCP product integration
- Dimension(s): 15
- pi entry: config discovery `src/extensions/mcp/config.ts:1-26` (`~/.pi/agent/mcp.json` + project `.pi/mcp.json` after trust; project entry replaces or patches `enabled/exposure/toolExposure` of the user entry); server model `core/mcp-servers.ts:1-120` (stdio + streamable HTTP; exposures `codemode|deferred|direct|hidden` with per-tool `toolExposure` patterns; per-request timeout default 60s; OAuth config incl. DCR/CIMD and loopback rules `mcp-servers.ts:70-140`); runtime `src/extensions/mcp/index.ts:1-24` (background connections, first prompt waits only for `direct` servers, tools `mcp__<server>__<tool>`, every call through pi's tool pipeline); resources `src/extensions/mcp/resources.ts` (`list_mcp_resources`, `list_mcp_resource_templates`, `read_mcp_resource`); manager UI `/mcp` and CLI `pi mcp add/list` (`docs/mcp.md:1-30`, `src/extensions/mcp/cli.ts:1`); `auth: {provider}` token forwarding (https-only, project files excluded)
- Opi entry: absence search `grep -rli 'mcp' crates/*/src` → only incidental matches: `crates/opi-ai/src/auth/oauth/anthropic.rs:31` (OAuth scope string), base64 blobs in `crypto/fixtures.rs`/`tls/node_trust.rs`, `opi-eval` transcript fixtures (`benchmark/task_lifecycle.rs:402`, `native_capture.rs:1222,1657`). No MCP client, config, or tool surface exists in any opi crate.
- Comparison: missing
- Observable difference: opi cannot connect to any MCP server; there is no `mcp.json`, no `pi mcp`-style CLI, no MCP tools/resources, no OAuth-for-MCP.

### EXT-26 — per-run evidence manifests (Opi-only)
- Dimension(s): 17
- pi entry: none (pi has no evidence-manifest contract)
- Opi entry: `crates/opi-coding-agent/src/evidence.rs:1-80` (`FileEvidenceSink`: `evidence.jsonl` + atomically published `manifest.json` per run; identities `opi.cli`/`opi.sdk`/`opi.rpc`; redacted values only); backed by `crates/opi-agent/src/evidence.rs` (Agent Core contract)
- Comparison: opi-only
- Observable difference: opi writes structured run evidence artifacts pi never produces; not a pi reproduction target but replaces part of pi's bug-report role with an audit-trail role.

### EXT-27 — doctor command surface (Opi-only)
- Dimension(s): 17
- pi entry: absence — `grep -rn 'doctor' .pi/pi-1.5.0/packages/coding-agent/src` and docs → no pi doctor command
- Opi entry: `crates/opi-coding-agent/src/doctor.rs:1-40` (`opi doctor [--json] [--scope config,provider,package,session,tui,rpc]`, network-free, credential presence only, exit 0/2/1 policy); `package doctor` in `package_cli.rs:123`
- Comparison: opi-only
- Notes: Layer H — `doctor.rs:11-13` states "Network checks are an explicit later design and intentionally absent" (designed-only extension point).

---

## PKG — packages

### PKG-01 — package source kinds
- Dimension(s): 12
- pi entry: `core/package-manager.ts:126-180` (sources: `npm:` spec with pinned/range semantics, `git:` URL/tag/commit, bare URL treated as git, local path resolved relative to the settings file); npm/git dependency install for git sources (`getGitDependencyInstallArgs:1955-1977`, `--omit=dev`, peer suppression); `-e` temporary cache under `~/.pi/agent/tmp/extensions/` with staleness checks (`getExtensionTempFolder:254`, `docs/packages.md:33-36`); package-manager selection `npmCommand` (npm/pnpm/bun, `package-manager.ts:1904-1936`)
- Opi entry: `crates/opi-coding-agent/src/package_store.rs:77-160` (`PackageSource::Local | Git` only; scp-like git URLs rejected with a redirect message); no npm source, no URL shorthand, no dependency installation, no `-e` temporary mode (`grep -rn 'npm' crates/opi-coding-agent/src/package_*.rs` → no npm usage)
- Comparison: partial
- Observable difference: `opi package add npm:...` is unsupported; git packages cannot carry installable runtime dependencies (Rust adapters are self-contained); no package-manager selection.

### PKG-02 — package manifest format and resource declaration
- Dimension(s): 12
- pi entry: `core/pi-manifest.ts:4-42` (`package.json` `pi.{extensions,skills,prompts,themes,changelogPath}`; arrays accept globs and `!`/`+`/`-` filters); conventional dirs + `pi-package` keyword gallery eligibility (`docs/packages.md:59-88`); settings-side object filters (`docs/packages.md` "Select package resources"); resource precedence `package-manager.ts:210-224` (project local > project auto > user local > user auto > package > builtin)
- Opi entry: `crates/opi-coding-agent/src/package_discovery.rs:116-160` (`package.toml`: `name` (required, lowercase/hyphen, ≤64), `description` (required, ≤1024), `version`, `opi_version` semver-ish constraint (advisory, `OpiVersionDiagnostic::check:254`), allowlists `extensions/skills/fragments/themes`, `disabled` excludes, `[adapter]` table, `[[contributions.adapters]]`); filters on declarations `package_store.rs:264-277` (`PackageFilters{extensions,skills,fragments,themes}`)
- Comparison: different
- Observable difference: opi requires a TOML manifest with a description; pi auto-discovers from conventional dirs with no manifest. pi filters support glob patterns and force-include/exclude overrides; opi filters are exact-name allowlists. pi packages can carry prompts; opi uses "fragments". opi adds `opi_version` gating and executable contributions pi has no analog for.

### PKG-03 — install/add flow
- Dimension(s): 12
- pi entry: `core/package-manager.ts:1090-1118` (`install`/`installAndPersist` write to settings `packages[]`), npm into a managed root with `--prefix`, `--legacy-peer-deps`, `--no-audit` (`getNpmInstallArgs:1979-2006`), git clone + checkout + optional dependency install with failure cleanup (`installGit:2008-2041`), project-trust gate for project scope (`assertProjectTrustedForScope:1887`)
- Opi entry: `crates/opi-coding-agent/src/package_cli.rs:130-180` local add: resolve + canonicalize path, `package.toml` required, `local_lock_entry` with manifest sha256, no copy; `package_cli.rs:187-380` git add: staging clone + `git rev-parse HEAD`, contribution validation before any live-state change, atomic cache replacement with rollback (`PendingCacheReplacement`, `package_store.rs:318-334`), metadata/trust snapshot restore on failure
- Comparison: different
- Observable difference: pi's npm path delegates integrity/rollback to npm; opi implements staging, hashing, atomic swap, and rollback itself for git and records a lock per package. Local packages: both reference the path without copying; opi additionally pins the manifest hash so edits invalidate the lock. pi persists declarations in JSON settings; opi uses `packages.toml`.

### PKG-04 — update flow
- Dimension(s): 12
- pi entry: `core/package-manager.ts:1144-1300` (`update(source?)`: registry version checks with concurrency 4, git ref reconciliation concurrency 4, pinned specs skipped, stale-registry "still X" warning `reportUnchangedNpmUpdates:1275-1295`; `pi update --extensions`, `pi update <source>`, aliases in `docs/cli.md:271-299`); offline mode `PI_OFFLINE`
- Opi entry: absence — `PackageCommand` enum has only Add/Remove/List/Doctor/Enable/Disable (`crates/opi-coding-agent/src/cli.rs:322-365`); `opi package add` re-run is the only refresh path
- Comparison: missing
- Observable difference: no way to update installed packages or check for newer versions.

### PKG-05 — remove and list flows
- Dimension(s): 12
- pi entry: `core/package-manager.ts:1119-1143` (`remove`/`removeAndPersist`: npm uninstall / git dir removal / local no-op + settings entry removal); `listConfiguredPackages:1062` (source, scope, filtered flag, installed path); `pi list` (`docs/cli.md:263`)
- Opi entry: `crates/opi-coding-agent/src/package_cli.rs:384-440` `cmd_remove` (by source or name, drops declarations/lock/trust records and cache dir); `cmd_list` with `--json` line output (`cli.rs:339-344`)
- Comparison: partial
- Observable difference: functionally parallel for the shared source kinds (local/git); opi adds JSON output and cache-dir cleanup, pi reports `filtered` state and installed path. pi removes npm-managed trees opi never creates.

### PKG-06 — per-package trust, enable/disable lifecycle (Opi-only)
- Dimension(s): 12
- pi entry: project trust only (`core/project-trust.ts`, `trust-manager.ts`); once a project is trusted all its packages/extensions load with no per-package confirmation
- Opi entry: `crates/opi-coding-agent/src/package_activation.rs:1-35` (`package-trust.toml`, execution packages global-only, first enable shows identity/version/locked-executable-hash/contributions and requires interactive confirmation, drift durably invalidates trust, pre-spawn revalidation `PackageActivationStore::activate`); `package_cli.rs:451-473` (`cmd_enable`/`cmd_disable`)
- Comparison: opi-only
- Observable difference: opi refuses to execute contributed executables until an explicit per-package enable; pi has no equivalent gate.
- Notes: Layer H — module doc cites spec lines 318-320/363-364 and Phases 16.5/16.7/16.9 (implemented per doc).

### PKG-07 — package doctor (Opi-only)
- Dimension(s): 12, 17
- pi entry: absence (see EXT-27)
- Opi entry: `crates/opi-coding-agent/src/package_cli.rs:123` (`cmd_doctor`, JSON option) over `package_resolver.rs` diagnostics (missing roots, lock mismatches, `opi_version` advisories)
- Comparison: opi-only

### PKG-08 — startup resolution and failure tolerance
- Dimension(s): 12
- pi entry: `core/package-manager.ts:974-1044` (`resolve`: project+global packages deduped, install-on-missing callback, `installFailures` skip-and-warn, retry next start/`/reload` per `docs/packages.md:19`); adapter-free: resources are files
- Opi entry: `crates/opi-coding-agent/src/runtime_packages.rs:34-82` (`start_installed_package_runtime_with_trust`: resolution diagnostics; adapter startup failures degrade to static resources and diagnostics; project scope gated on `TrustDecision`); adapter spawn/timeout failure table `adapter_protocol.rs:59-72`
- Comparison: partial
- Observable difference: both skip-and-report instead of crashing, but opi never retries installation automatically (pi re-attempts on next start/`/reload`), and opi's failure can also mean a degraded adapter (tools/commands unavailable) rather than missing files.

### PKG-09 — package changelog surfacing
- Dimension(s): 12
- pi entry: `pi.changelogPath` (`core/pi-manifest.ts:23`, `core/package-manager.ts:707-725` path containment checks); "Extension Updates" section on next fresh launch, version-heading matching, `collapseChangelog` setting (`docs/packages.md:91-113`)
- Opi entry: absence — no changelog reading/display in `package_cli.rs`/`runtime_packages.rs`
- Comparison: missing

### PKG-10 — self-update / managed installs
- Dimension(s): 12
- pi entry: `src/package-manager-cli.ts:55-120` (managed-install root marker, releases-v1 layout, `proper-lockfile` update lock `:185-203`, staged `npm ci --ignore-scripts`, quarantine for Windows native deps); `pi update` self with `PI_SELF_UPDATE_COMMAND`/`selfUpdateCommand` (`docs/cli.md:271-299`)
- Opi entry: absence — no self-update path anywhere in `crates/opi-coding-agent/src`
- Comparison: missing

### PKG-11 — install lock files
- Dimension(s): 12
- pi entry: npm's own `package-lock.json` inside the managed install roots (indirect via `npm install --prefix`); `proper-lockfile` used only for auth store (`auth-storage.ts:14`) and managed self-update; git packages keyed by directory layout with update markers (`getGitUpdateMarkerPath`)
- Opi entry: `crates/opi-coding-agent/src/package_store.rs:292-443` (`package-lock.toml`: `identity_kind/identity_value`, source, `package_root`, `manifest_sha256`, git commit + cache path; atomic staged writes; `package_resolver.rs:211` `manifest_sha256`)
- Comparison: different
- Observable difference: opi's lock is a first-class opi artifact pinning content hashes; pi's pinning lives inside npm metadata and git checkouts. Both exist for reproducibility, but lock identity, granularity, and failure behavior differ.

---

## Coverage honesty

- Read fully or near-fully: pi `core/extensions/types.ts`, `core/mcp-servers.ts`, `core/model-config.ts`, `core/pi-manifest.ts`, small diagnostics modules (`diagnostics.ts`, `telemetry.ts`, `client-identity.ts`, `bug-report-upload.ts`, `runtime-credentials.ts`, `auth-guidance.ts`), `docs/extensions.md` (first ~1123 lines + section index), `docs/packages.md`, `docs/mcp.md` (first 80 lines), `docs/models-configuration.md` (first 110 lines); opi `adapter_extension.rs`, `adapter_protocol.rs`, `runtime_packages.rs`, `diagnostics.rs`, `resource.rs` (manifest parts), `package_store.rs`/`package_discovery.rs`/`package_activation.rs`/`package_cli.rs` (structure + install/remove paths), `opi-agent/src/extension.rs`.
- Read selectively (headers, exports, key functions): pi `core/extensions/loader.ts` (all but middle), `core/package-manager.ts` (~60%: resolve/install/update/trust; not every git helper), `core/provider-composer.ts`, `core/model-runtime.ts`, `core/virtual-models.ts`, `core/remote-catalog-provider.ts`, `core/bug-report.ts`, `core/crash-log.ts`, `core/auth-storage.ts`, `core/cache-stats.ts`, `core/timings.ts`, `core/footer-data-provider.ts`, `experimental/source-resolver.ts`, `src/extensions/{codemode,tool-search,llama,mcp}` (module docs + key files), `src/package-manager-cli.ts`; opi `provider_factory.rs` (~15%: bundle/collection/listing/overrides), `credential_store.rs`/`native_keyring.rs` (contract docs), `doctor.rs`/`evidence.rs`/`diagnostic_bridge.rs` (module docs + surface), `github_copilot.rs`/`openai_codex.rs` (headers), `config.rs` (provider structs).
- Not inspected: pi `core/extensions/runner.ts` internals (event dispatch implementation), `src/extensions/mcp/index.ts` full runtime logic, MCP OAuth implementation details (`extensions/mcp/oauth.ts`), pi settings-manager details for package settings merge, opi interactive/RPC wiring beyond `dispatch_extension_command` and auth prompts, opi-eval MCP-adjacent fixtures, tests on either side, and any built artifacts (none exist per inputs). No executed probes (read-only batch).
- Dimension numbers are asserted from the brief header mapping (7=extensions, 12=packages, 3=model config, 15=MCP, 17=diagnostics); the authoritative 20-dimension list was not independently re-derived.
