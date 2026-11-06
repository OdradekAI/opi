# Batch AI-MOD — model catalog & provider registry (Dims 1,3,16)

Contracts measured: builtin provider registry (ids/auth/baseUrls), per-provider model
catalog + ModelType coverage, ModelInfo/Model record fidelity, pricing + thinking
semantics, catalog generation/pinning/hydration, models-store persistence + refresh
semantics (ETag/offline/supersession), remote-catalog overlay gating, radius dynamic
provider, custom/user model overlays, anthropic + openai deep dives.
Verdict tallies (16 records): equivalent 6, partial 5, different 3, missing 1,
opi-only 0, unresolved 1.
Biggest gaps: no pi.dev remote-refresh revalidation/freshness gating (ETag/304/4h
window/lastModified-vs-generatedAt) in opi's HTTP source; radius gateway dynamic
catalog fetch absent; opi bundle pins the pi 1.3.0 revision (ffa3e98…) while pi 1.5.0
pins f4dbabe… (content parity unprovable offline).
Uncertainties: exact per-provider model-count parity pi-vs-opi (pi export lacks
generated `data/*.json`); promptCache effect on opi cache warming lives outside this
batch's scope.

---

### AI-MOD-01 — Builtin provider registry enumeration
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/providers/all.ts:136` (`builtinProviders()`, 42 factories; ids also in `models.generated.ts:47` MODELS map) — public.
- Opi entry: `crates/opi-ai/src/providers/mod.rs:70` (`BUILTIN_PROVIDER_PROFILES`, 42 entries; `builtin_provider_profiles()` at :59) — public.
- Comparison: equivalent
- Observable difference: none in the id set: set-diff of the 42 pi factory ids vs the 42 opi profile ids is empty (camelCase↔kebab only). Spot-checked names/baseUrls/env keys match: `anthropic` (no env key, ApiKey, oauth) `providers/anthropic.ts:87-92` vs `providers/mod.rs` anthropic block; `openai-codex` "OpenAI Codex (legacy)" `https://chatgpt.com/backend-api` oauth-only `providers/openai-codex.ts:7-9` vs opi profile; env map (`env-api-keys.ts:70-115`: HF_TOKEN, COPILOT_GITHUB_TOKEN, GEMINI_API_KEY, OPENCODE_API_KEY shared by opencode/opencode-go, MOONSHOT_API_KEY shared by moonshotai/-cn, QWEN_TOKEN_PLAN_API_KEY shared by plan/individual) matches opi `api_key_env` values. opi encodes endpoint semantics (Azure deployment, Vertex location, Bedrock region, per-model base URL, dynamic) as data; pi encodes them in factory code — representational only.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's table header says "pinned … pi 1.3.0 capability inventory" (`providers/mod.rs:1-3`) yet the id set equals pi 1.5.0's. pi `google-vertex` additionally accepts `GOOGLE_CLOUD_API_KEY` as an env key (`env-api-keys.ts:88`); opi's google-vertex profile has no env key (ADC chain label only) — ambient-auth chain equivalence is out of this batch's depth.

### AI-MOD-02 — Per-provider model counts & ModelType coverage
- Dimension(s): 1, 16
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models.generated.ts:47,135,223` (MODELS/IMAGE_MODELS/CLASSIFIER_MODELS over `src/providers/data/<id>.json`) — public, but the data files are absent from the export (`ls .pi/pi-1.5.0/packages/ai/src/providers/data/` → not found; `find .pi/pi-1.5.0/packages/ai -name '*.json'` → only package/tsconfig/test data).
- Opi entry: `crates/opi-ai/resources/catalog/builtin/catalog.json` (42 providers, 1705 entries: 1608 chat / 59 image / 38 classifier; counted via python over the artifact), loaded by `crates/opi-ai/src/catalog/mod.rs:421` `Catalog::builtin`.
- Comparison: unresolved
- Observable difference: counts per provider cannot be computed on the pi side (generated data absent per 00-inputs.md). Opi's counts come from a real pi-published artifact (manifest `crates/opi-ai/resources/catalog/builtin/manifest.json`: revision sha256-ffa3e98…, schema v6). Both sides cover all three model types with the same identity scheme (`type:id`): pi `catalog/generate.ts` sorted-by-id grouping vs opi `CatalogModel::identity()` (`catalog/mod.rs:340-343`) and `wire.rs:1347` decode (`type` default "chat", image/classifier enumerated).
- Evidence basis: source inspection (artifact counted; pi data unavailable)
- Review outcome: self-reviewed
- Notes: pi 1.5.0's own pin is `.pi/pi-1.5.0/nix/model-catalog.json` revision sha256-f4dbabe… ≠ opi's ffa3e98… (see AI-MOD-07). Classifier APIs both sides: `typesafe-system-one`, `cloudflare-workers-ai-system-one`, `llama-cpp-classify`, `openai-decisions` (`types.ts:82-88`) vs opi api modules `crates/opi-ai/src/api/{system_one,cloudflare,llama,decisions}.rs`; image API `openrouter-images` both sides.

### AI-MOD-03 — Model record fields (ModelInfo vs Model)
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/types.ts:838-883` (`BaseModel`: id/name/api/provider/baseUrl/input/inputLimits/cost/headers; `Model`: type?/reasoning/thinkingLevelMap/promptCache/contextWindow/maxTokens/samplingParams/samplingParamsByThinkingLevel/compat; `ImageModel`: output modalities; `ClassifierModel`: contextWindow+compat).
- Opi entry: `crates/opi-ai/src/model_info.rs:1226` (`ModelInfo`: id/display_name/wire_api/capabilities/base_url/thinking_level_map/compat/pricing/sampling_params/sampling_params_by_thinking_level/headers/input_limits); image/classifier records `crates/opi-ai/src/images.rs:119`, `crates/opi-ai/src/classifier.rs:283`.
- Comparison: partial
- Observable difference: (a) `promptCache {short,long}` lifetime seconds exist in pi and in the bundled artifact (e.g. anthropic entries carry `"promptCache":{"short":300,"long":3600}`) but opi's `ModelInfo` has no field and `catalog/wire.rs` never reads the key (absence search: `grep -rn "promptCache" crates/opi-ai/src/` → no decode); opi substitutes two booleans `supports_cache_control`/`supports_long_cache_retention` defaulted by an anthropic-wire heuristic (`wire.rs:1080-1083`). (b) pi carries `provider` and `input` modalities on every record; opi chat `ModelInfo` has neither (owner is external; `supports_images` bool at `wire.rs:1067-1071` from `input`), though image/classifier records do carry `provider_id`/`input`. (c) `inputLimits` is retained verbatim as raw JSON (`wire.rs:895-897`) — pi applies it in preprocessing; opi documents it as non-policy metadata.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: promptCache loss is the concrete fidelity gap; whether opi's agent layer reconstructs equivalent cache-warming timing from the two booleans belongs to the agent batch. `name`→`display_name`, required `baseUrl`→`Option<String>` with profile fallback are representational.

### AI-MOD-04 — Pricing fields, units, tier and service-tier cost math
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/providers/model-schema.ts:44-63` (per-Mtok USD rates input/output/cacheRead/cacheWrite + `tiers[].inputTokensAbove` request-wide highest-matching); `models.ts:1236` `calculateCost` (cacheWrite1h at 2× input); codex/responses service-tier multipliers `api/openai-codex-responses.ts:619-643` (flex 0.5, priority 2/2.5-for-gpt-5.5, fast→1), `api/openai-responses.ts:426-445` (priority|fast → 2/2.5).
- Opi entry: `crates/opi-ai/src/model_info.rs:968` (`ModelPricing{base,tiers}`, `effective_for_usage` at :1004 counting input+cacheRead+cacheWrite, strictly-exceeded threshold); `calculate_cost_tiered` at :1163 (1h subset at 2× input, responses+codex wires only; Codex `Fast` pinned to 1.0); `stream.rs:91-98` `cost_multiplier` (flex 0.5, priority|fast 2.0, gpt-5.5 2.5).
- Comparison: equivalent
- Observable difference: none found in the representatives: same per-Mtok units, same request-wide tier selection, same cacheWrite1h doubling, same flex/priority/fast multipliers per wire including the Codex-fast exception. opi additionally returns `None` (unknown cost) for unreported usage/non-finite placeholders vs pi always producing numbers — fail-closed refinement, same inputs produce same finite outputs.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi keeps finite negative source placeholders as metadata (`validate_source_metadata` :979) mirroring pi's generator-validated data.

### AI-MOD-05 — Thinking levels and clamping
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models.ts:1259-1292` (levels off..max; `getSupportedThinkingLevels`: no reasoning→["off"], xhigh/max only when explicitly mapped, null unsupported; `clampThinkingLevel` upward-then-downward); `providers/model-schema.ts:16-36` ThinkingLevelMap (string|null values).
- Opi entry: `crates/opi-ai/src/model_info.rs:172-200` (ThinkingLevel::ALL 7); `ThinkingLevelMap::disabled`/`reasoning_default` :243-269; `nearest_supported` :315 (exact→higher→lower, cites pi `clampThinkingLevel`); map decode `catalog/wire.rs:875-889` (null→Unsupported, string→Mapped).
- Comparison: equivalent
- Observable difference: none in level set, mapping semantics (bundled `gpt-5.5` `{"off":"none","minimal":null,...}` decodes identically on both sides), or clamp order.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi adds `has_explicit_mappings` (:283) as an adapter-facing projection aid; not observable in catalog data.

### AI-MOD-06 — Catalog generation & hydration pipeline
- Dimension(s): 3
- pi entry: `.pi/pi-1.5.0/packages/ai/catalog/generate.ts:1-160` (fetch upstream sources → build → write gitignored `src/providers/data/*.json` + committed `.models.ts` + `models.generated.ts` + `.manifest.json`), `scripts/hydrate-model-catalog.ts:39-77` (offline hydration from published `models.all.json`, fixed 1970 stamp), `scripts/model-data.ts:5` (MODEL_DATA_SCHEMA_VERSION 6, structure validation), `scripts/check-model-data.ts`.
- Opi entry: `scripts/pi-catalog/pack.py:1-40` (never generates; verifies a downloaded artifact byte-for-byte against the pinned revision and validates with the same rules as pi's model-data.ts/hydrate-model-catalog.ts, then emits `resources/catalog/builtin/{catalog,manifest}.json`); loader `crates/opi-ai/src/catalog/mod.rs:445` `Catalog::from_bytes` (digest + structure + per-entry re-verification, fail-closed `CatalogError`).
- Comparison: different
- Observable difference: pi's repo pipeline regenerates typed catalogs from upstream (models.dev/OpenRouter/Vercel/NVIDIA/Radius sources under `catalog/sources/`) and hydrates offline from a published document; opi has no generator/hydrator — absence search: `grep -rn "generate-models\|hydrate" scripts/pi-catalog/` (pack.py only references pi scripts as validators) and no `catalog/` generator exists under `crates/opi-ai/`. opi instead embeds one immutable published revision and re-validates at load. Same data contract (schema v6, structure digest) is enforced on both sides.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's manifest records sha256 of pi's `model-data.ts` (f2605a61…) and `hydrate-model-catalog.ts` (153433c8…); recomputing them over the pi 1.5.0 snapshot gives byte-identical digests — the validation contract is in sync with 1.5.0 even though the packer pins `.repo/pi-1.3.0` (pack.py:29-33).

### AI-MOD-07 — Catalog revision pinning
- Dimension(s): 3
- pi entry: `.pi/pi-1.5.0/nix/model-catalog.json:1` (`"revision": "sha256-f4dbabe79cda4b38aee4a761b23fd5eaf526ac2e28fed3c9ee86e37ca0e5d71e"`); scripts `scripts/model-catalog-protocol.ts`, `model-catalog-minimum-version.mjs` govern published-revision protocol.
- Opi entry: `crates/opi-ai/resources/catalog/builtin/manifest.json` (`revision: sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678`, `source.local_origin: ".repo/pi-1.3.0"`, `pi_package: "@earendil-works/pi-ai@1.3.0"`).
- Comparison: different
- Observable difference: opi ships a different (older-pinned) catalog revision than the one pi 1.5.0 pins. Content deltas cannot be proven offline (both revisions are remote artifacts); spot checks show the opi bundle already contains pi-1.5.0-era entries (claude-fable-5-1, gpt-6.1-sol, gpt-5.6 family with the 1.5.0 finalize rules: `off:"none"`, gpt-5.5 `minimal:null`, `supportsExplicitPromptCacheMode` iff cacheWrite>0, `supportsMidConvoEffort` on gpt-6-sol), so the drift may be small — but identity differs and any models added/changed between ffa3e98 and f4dbabe are absent from opi.
- Evidence basis: source inspection (manifests + artifact spot checks)
- Review outcome: self-reviewed
- Notes: open question for pass 2: fetch both revisions (network) and diff; prohibited in this audit.

### AI-MOD-08 — Models-store persistence semantics
- Dimension(s): 3
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models-store.ts:3-46` (`ModelsStoreEntry{models,lastModified,checkedAt,etag}` verbatim-etag contract; `ModelsStore` per-provider read/write/delete; default `InMemoryModelsStore`); product file store `.pi/pi-1.5.0/packages/coding-agent/src/core/models-store.ts:50` (`FileModelsStore`, locked JSON at agentDir/models-store.json with file revision + shared read state).
- Opi entry: `crates/opi-ai/src/catalog/mod.rs:154` (`ModelsStoreEntry{models,lastModified,checkedAt,etag}` same field set, "SDK does not invent a refresh schedule or interpret the opaque ETag"); `catalog/store.rs:117` `ModelsStore` (single JSON document, u64 revision, `RefreshGuard` :265 commit-only-if-current, atomic temp+rename, `StoreCommitError::Superseded/Cancelled` :66, publisher tags + `invalidate_source` :205).
- Comparison: partial
- Observable difference: entry fields and offline-restore-on-open match. pi's store is a per-provider interface with product-owned file locking in coding-agent; opi's is one SDK-owned document with explicit revision guards, publisher provenance, and source tombstones — opi adds supersede/cancel rejections at store level, pi achieves supersession in `Models` (AI-MOD-09). opi `set_models_store` (`models.rs:3393`) installs lazily ("restores happen at the next refresh round") vs pi constructor injection; first read timing differs for a caller inspecting before any refresh.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: layering difference (pi product vs opi SDK) is architectural, but the revision/publisher/tombstone semantics are opi-only observable behavior.

### AI-MOD-09 — Refresh semantics (restore→fetch, supersession, per-provider errors)
- Dimension(s): 3
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models.ts:556-646` (`refresh`: per-provider generations, `supersedeProviderRefresh` :498 aborts prior controllers, chained per-provider publication :516, restore phase (allowNetwork=false) then credential resolve then network phase, `Promise.all` concurrency, errors Map, `withKnownModelTypes` :140 silently drops unknown model types).
- Opi entry: `crates/opi-ai/src/models.rs:3008` `refresh_with` (offline restore from store snapshot → sequential per-provider `refresh_models` → sequential sources → single `commit_with_publishers`; `RefreshStatus::Superseded` via `publication_generation` Arc-ptr :3216 and store revision; `RefreshReport` entries `models.rs:441-528` with Restored/Published/Failed outcomes; unknown model types rejected: `wire.rs:1362` "unsupported model type").
- Comparison: partial
- Observable difference: core lifecycle equivalent (restore-before-network, failed owners retain prior records, stale publications cannot land, per-owner error isolation). Differences: (a) pi refreshes providers concurrently, opi sequentially; (b) pi publishes per provider as each finishes, opi commits once at the end (all-or-nothing persistence); (c) unknown future model types: pi filters them out and keeps the rest, opi fails the whole entry/provider (fail-closed); (d) opi's report exposes Restored counts and OfflineRestored/Superseded terminal states that pi's `{aborted, errors}` does not; (e) opi additionally gates every dynamic chat model through `chat_model_admitted` (registered-route validation, `models.rs:3252-3268`) — pi has no equivalent admission gate.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: (a)-(d) are observable to concurrent callers and to mixed-type catalog consumers; deliberate/safer choices still count as different under the strict lens, hence `partial` for the shared core with listed divergences.

### AI-MOD-10 — Remote catalog refresh overlay (ETag/304/interval/stale-generation gate)
- Dimension(s): 3
- pi entry: `.pi/pi-1.5.0/packages/coding-agent/src/core/remote-catalog-provider.ts:104-186` `withRemoteCatalog`: 4h skip window (:16,131-141), `if-none-match` revalidation only when a cached body backs the validator (:88-91,147-152), 304 → persist checkedAt only, 404/501 → clear lastModified/etag, transient failure keeps etag (:160-170), overlay applied only when `lastModified > localGeneratedAt` (`isNewer` :67-73; localGeneratedAt from `getBuiltinModelDataGeneratedAt` `providers/all.ts:98-103` parsing `data/.manifest.json`); `withPersonalizedCatalog` :195+ for credential-specific lists.
- Opi entry: `crates/opi-ai/src/catalog/http_source.rs:79-171` `HttpCatalogSource` — plain GET of one distribution-schema-v1 document, 30s timeout, non-2xx/transport → redacted ProviderError; no If-None-Match, no 304 handling, no refresh interval, no lastModified-vs-local comparison. Absence search: `grep -rni "if-none-match\|etag\|304" crates/opi-ai/src/` → only the passthrough `etag` field (`catalog/mod.rs:142`, `wire.rs:1579-1585`), never used for revalidation or gating.
- Comparison: different
- Observable difference: a pi caller gets conditional revalidation, bandwidth-free 304s, bounded re-check frequency, and immunity to an older-than-shipped remote catalog; an opi caller re-downloads the full document every refresh and applies whatever the source returns regardless of its age relative to the bundled revision. Opi's `ModelsStoreEntry.last_modified/etag` are stored but never drive any decision.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi's SDK deliberately registers no default source (`catalog/http_source.rs:5-8`); the product could layer pi.dev semantics, but nothing in opi-ai implements them. OpenRouter personalization IS reproduced (see AI-MOD-16 note): `catalog/openrouter.rs:22` matches pi `personalizeOpenRouterCatalog` (`providers/openrouter.ts:23-73`) on 401/403/404 passthrough, per-key filtering, positive-limit `||` fallback semantics, 15s deadline — and adds pricing-override→tier mapping pi 1.5.0's helper leaves to the generator.

### AI-MOD-11 — Radius dynamic gateway provider
- Dimension(s): 1, 3
- pi entry: `.pi/pi-1.5.0/packages/ai/src/providers/radius.ts:17-100` (baseline from `radius.models.ts` only for the default gateway; `refreshModels` restores stored → imports legacy pre-ModelsStore cache (:59-72) → `loadRadiusGatewayConfig` + `getRadiusModelsFromConfig` fetch, org-disabled models replace baseline; dynamic list wins over baseline in `getModels` :40).
- Opi entry: `crates/opi-ai/src/pi_messages.rs:540-555` (opi-curated one-model baseline `claude-sonnet-4-5` for the default gateway, comment cites pi `radiusProvider.refreshModels`) + generic registration path `provider_collection.rs:851-872` (PiMessages route over bundled catalog's 29 radius chat entries); default `refresh_models` returns `Ok(None)` (`provider.rs:199-207`, `api_mapped.rs:332-340`).
- Comparison: missing
- Observable difference: no opi code fetches a configured gateway's model list: absence search `grep -rn "getRadiusModelsFromConfig\|loadRadiusGatewayConfig\|gateway_config" crates/ --include="*.rs"` → 0 matches; `grep -rn "radius" crates/opi-ai/src/` → only pi_messages baseline, oauth flow (`auth/oauth/radius.rs`, pinned to pi 1.3.0) and profile table. A pi user pointing radius at a custom gateway gets that gateway's live catalog; an opi user gets the static bundled/baseline list. Legacy cache import has no counterpart either.
- Evidence basis: source inspection (bounded absence search above)
- Review outcome: self-reviewed
- Notes: pi's per-account replacement (dynamic replaces rather than overlays) is also not reproducible without the fetch; opi's registry would support it via dynamic catalogs once a source exists.

### AI-MOD-12 — Custom/user model overlays
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models.ts:1077-1160` (`createProvider` merges dynamic overlay over baseline by (type,id); used by builtin factories AND models.json custom providers) + product schema `packages/coding-agent/src/core/model-config.ts:44-152` (models.json: per-provider `models[]` with defaults (contextWindow 128000, maxTokens 16384), `modelOverrides` per model with partial cost/promptCache, `$env`/`!command` apiKey interpolation, api/baseUrl overrides applying to built-in models, `oauth: "radius"`).
- Opi entry: `crates/opi-ai/src/registry.rs:179` `register_model` (override layer shadows built-ins and dynamic catalogs on collision; duplicate override → `DuplicateModel`), `registry.rs:111` `register_provider` (same-id replacement clears dynamic catalog); product overlay `crates/opi-coding-agent/src/config.rs:300-321` (`[providers.custom.<id>]` TOML: 3 wires only — anthropic-messages/openai-completions/openai-responses — env-var key (`api_key_env`), model values win over provider defaults) via `provider_factory.rs:187`.
- Comparison: partial
- Observable difference: precedence core matches (override > dynamic > builtin on both sides; pi `models.ts` merge + `MutableModels.setProvider` upsert; opi registry layering `registry.rs:225-248`). Gaps: opi has no per-model partial overrides of *builtin* provider models from config (only whole `ModelInfo`s on custom providers or programmatic `register_model`), no `$env`/`!command` secret interpolation (env-var name only), no `oauth: radius` custom-provider flag, no promptCache/inputLimits authoring surface, and no per-model default application (defaults live in code).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: models.json vs TOML is a product-config choice; the measurable deltas are the override granularity and secret-resolution forms.

### AI-MOD-13 — Anthropic representative deep dive
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/catalog/providers/anthropic.ts:1-99` (1M-context overrides for opus-4-6/sonnet-4-6; claude-5-5 thinking map; `promptCache {short:300,long:3600}`; inputLimits 32MB + 100/600 images; `allowedFallbackModels` with priced fallbacks filtered by mid-convo-effort support; strict tools; mid-convo system/tool changes) + `providers/anthropic.ts:15-86` (auth-token → oauth-token → api-key → workload-identity-federation env chain).
- Opi entry: bundled entries carry all catalog facts verbatim (spot check `claude-opus-5`: promptCache/inputLimits/thinkingLevelMap/compat incl. `allowedFallbackModels` for fable-5/opus-5 targets); decode `catalog/wire.rs:1031-1120` (`decode_model_entry` + `apply_source_metadata`: thinkingLevelMap, inputLimits raw, headers, compat incl. `allowed_fallback_models` :392-399 with ≤3 fallback validation `model_info.rs:1352-1366`); anthropic auth chain `crates/opi-ai/src/auth/` (profile `api_key_env: None`, oauth: true) with AnthropicFederation scheme in `AuthScheme`.
- Comparison: partial
- Observable difference: catalog data fidelity is high (same values, same fallback pricing usable for response-model cost attribution — opi `calculate_response_cost` `model_info.rs:1291-1317` consumes fallback pricing like pi's fallback billing). Divergences: promptCache lifetimes dropped at ModelInfo (AI-MOD-03); inputLimits (resize profile, 32MB request cap, image counts) retained but explicitly non-policy, so preprocessing behavior driven by these limits is a caller choice, not catalog-driven as in pi.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: opi `AnthropicMessagesCompat` (`model_info.rs:331-370`) reproduces pi's anthropic compat flag set one-to-one (eager streaming, long retention, cache-control-on-tools, adaptive thinking, temperature, empty signature, strict tools, mid-convo system/tools/effort, affinity, fallbacks).

### AI-MOD-14 — OpenAI representative deep dive (openai + openai-codex)
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/providers/openai.ts:7-34` (responses api; `filterAllModels` hides classifiers under ChatGPT oauth; decisions classifier api) + `catalog/providers/openai.ts:14-142` (UNSUPPORTED gpt-5.6; short-context capping at 272000/128000; long-context tiered pricing; NONE_REASONING `off:"none"`; gpt-5.5 `minimal:null`; strict mode, grammar tools, tool search, mid-convo system/effort, explicit prompt-cache mode iff cacheWrite>0; inputLimits 512MB/1500 images) + `providers/openai-codex.ts:6-10`.
- Opi entry: `providers/mod.rs` openai/openai-codex profiles (oauth true; `hides_classifiers_on_oauth: true` enforced `auth/builtin.rs:165,305`); bundled entries match the finalize rules (gpt-5.4 `{"off":"none","minimal":null,...,"max":null}`; gpt-5.6-sol `max:"max"` + `supportsExplicitPromptCacheMode`; gpt-6-sol `supportsMidConvoEffort`; gpt-5-pro maxTokens 128000; tiered costs on 5.4/5.5/5.6/6.x); codex route `crates/opi-ai/src/openai_codex_responses.rs` + `responses_path "/codex/responses"`, `derive_account_id` (`model_info.rs:940-951`).
- Comparison: equivalent
- Observable difference: none found in the sampled catalog outputs, profile identity (ids/names/baseUrls/env/oauth), or the oauth-classifier-hiding policy. Bundled openai set (44 chat incl. gpt-5.5/5.6-*/6-*/6.1-sol + gpt-6-luna decisions classifier) mirrors the pi 1.5.0 generator's model universe; `gpt-5.6` absent on both sides.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: full set equality still depends on revision parity (AI-MOD-07); verified by spot checks only.

### AI-MOD-15 — Auth-filtered availability (getAvailable family)
- Dimension(s): 1
- pi entry: `.pi/pi-1.5.0/packages/ai/src/models.ts:294-296,715-765` (`getAvailable`, `getAvailableOfType`, `getAllAvailable`; provider `filterModels`/`filterAllModels` applied after auth-configured check).
- Opi entry: `crates/opi-ai/src/models.rs:2073` `get_available_of_type`, `:2138` `get_all_available`, `:1960` `get_models`, `:1986` `get_models_of_type`; classifier availability `:1335` `get_available_classifiers`; image availability per registration `:2099-2126`.
- Comparison: equivalent
- Observable difference: same triad of operations over the three model types, same "configured auth required" gate, same oauth classifier-hiding outcome for openai (pi `filterAllModels` vs opi `hides_classifiers_on_oauth` policy in `auth/builtin.rs`). opi's image path additionally treats `responses_only` auth status as unavailable for images (:2108) — pi has no responses_only concept in this path; no pi behavior is lost.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes:opi enumerations return records without pi's `Model.provider` field (owner carried externally), consistent with AI-MOD-03.

### AI-MOD-16 — Image & classifier model records and operations
- Dimension(s): 16
- pi entry: `.pi/pi-1.5.0/packages/ai/src/types.ts:885-905` (`ImageModel{type:"image", output:("text"|"image")[]}`, `ClassifierModel{type:"classifier", contextWindow, compat}`); operations `generateImages`/`classify` on Models (`models.ts:331-346`) and Provider.
- Opi entry: `crates/opi-ai/src/images.rs:119-142` `ImageModelRecord` (output modalities with documented openrouter image-then-text request rule; cost + tiers-as-metadata), `crates/opi-ai/src/classifier.rs:283-301` `ClassifierModelRecord` (context_window, priced with tiers); registration plans `provider_collection.rs:1077-1090` (typesafe/openrouter/opencode/vercel → typesafe-system-one; openai → openai-decisions; cloudflare-workers-ai → system-one) matching the bundled classifier distribution (38 entries across exactly those providers).
- Comparison: equivalent
- Observable difference: none in record content or per-provider classifier coverage. opi documents that image cost tiers are retained but unused for image pricing (flat base rates) — pi's image cost handling is likewise rate-based without tiers.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: `llama-cpp-classify` exists on both sides as an API without builtin catalog entries (extension-owned in pi; `api/llama.rs` in opi).

---

## Layer H — active design material encountered

- H1: opi `crates/opi-ai/src/providers/mod.rs:1-3` — table is "pinned builtin provider profile table … from the pi 1.3.0 capability inventory". Authority wording: normative module doc (data table). Maturity: implemented (drives registration). Risk anchor: pinning language names 1.3.0 while this audit's target is 1.5.0; id set verified equal, but baseUrls/env/oauth were transcribed at 1.3.0 (spot checks pass).
- H2: opi `crates/opi-ai/src/catalog/mod.rs:30-34` and `registry.rs:56-59` — "# Unstable / unstable 0.x extension surface. Breaking changes may occur between minor versions". Authority wording: normative API contract marking. Maturity: implemented, explicitly unstable.
- H3: opi `crates/opi-ai/src/catalog/store.rs:11-14` — "The SDK never attaches a store or fetches a remote catalog by default: there is no pi.dev or default startup policy here." Authority wording: normative design decision. Maturity: implemented; this is the deliberate divergence behind AI-MOD-10 (pi's product layer owns pi.dev refresh).
- H4: pi `.pi/pi-1.5.0/packages/ai/catalog/README.md` — normative pipeline description (sources/providers/families/build order; "Prefer fixing wrong upstream data in models.dev"). Maturity: implemented tooling. No experimental markers found in pi's catalog/models files (no TODO/FIXME with design content in scanned scope).

## Coverage honesty

- Did NOT inspect: pi's `api/` wire adapters beyond pricing/tier/service-tier excerpts (owned by the wire-protocol batches); opi's wire adapters (`anthropic.rs`, `openai_*.rs`, `gemini.rs`, `vertex.rs`, `mistral.rs`, `bedrock/`) beyond registration wiring; auth resolution internals (OAuth flows, credential stores) beyond the profile-level facts needed for registry comparison; `opi-coding-agent` product behavior beyond locating the custom-provider overlay surface; pi coding-agent's model-registry/model-runtime orchestration beyond `remote-catalog-provider.ts` and `model-config.ts`; `catalog/families/*` and `catalog/sources/*` in detail (only their outputs in the bundled artifact); `catalog/build.ts`/`define.ts` internals.
- Could NOT measure: exact per-provider model-count parity (pi export lacks generated `data/*.json` — 00-inputs.md artifact-availability note) and content equality of catalog revisions f4dbabe vs ffa3e98 (network prohibited). Both recorded as unresolved with the evidence that exists.
- Executed no probes and ran no builds/tests (read-only inspection, per input constraints). All absence claims carry the grep/find commands in-record.
