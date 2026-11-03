## Unreleased

### Breaking Changes

- `opi-ai` (0.x): Chat compat uses typed routing, cache and affinity controls;
  `vllm_priority` becomes `Option<serde_json::Number>`, and template args/kwargs
  become optional maps (`None` inherits; an explicit empty map clears).
  Responses families share eleven optional source controls through
  `OpenAiResponsesSourceCompat`. Struct literals and adapter configurations
  migrate together; existing product TOML integer keys retain their values.
- `opi-ai` (0.x): `SpawnedCredentialProcess::policy()` returns
  `Option<CredentialProcessPolicy>`. The default SDK runner selects `None`,
  imposing no implicit timeout or stdout limit; `with_policy(Default::default())`
  retains the explicit 10-second/64-KiB bounds selected by the coding agent.
  Cancellation and owned process-tree cleanup apply in both modes.
- `opi-ai` (0.x): `Models::refresh` and `refresh_with` take
  `ModelsRefreshOptions` for network permission, provider selection and optional
  force. Provider refresh and dynamic-source fetch hooks take
  `CatalogRefreshContext`; dynamic sources declare their provider owners, and
  `HttpCatalogSource` constructors require those IDs explicitly.
- `opi-ai` (0.x): `Request` gains `projection: ProjectionOptions` with separate
  temporal and replay policies; `AssistantMessage` gains optional projection
  facts (old serialized records remain `None`/unknown), and `PreparedRoute`
  carries those facts. `resolve_transcript` returns
  `TranscriptProjectionOutput { context, facts }`. Responses strict, grammar
  and mid-conversation-effort controls move under the shared
  `OpenAiResponsesSourceCompat` owner through `source`, preserving existing
  values. Rust callers and bilingual README examples migrate together.
- `opi-ai` (0.x): retired the shallow SDK shells after slice-by-slice caller
  migration reached zero in-repo consumers (task 20.56, DEL-13): `Config`,
  `ProviderConfig`, `config::Error`, `config::Result`, `Model` and
  `ProviderKind` are removed. The typed replacements (`Models`,
  `ChatModelRecord`/`ModelInfo`, `ApiKind`, the `ProviderError` taxonomy)
  already cover every retained capability, and the constructor-only
  `provider_kind_variants` assertion was deleted with its shell; historical
  v1 read-only readers and sessionv2 writer facts are unchanged. The
  bilingual crate README is reorganized onboarding-first (install, quick
  start, streaming/final result, Context/tools/continuation,
  factories/catalog/auth, options/errors/cancel/usage) with three new
  compiling examples (`examples/quick_start.rs`, `quick_start_stream.rs`,
  `tool_loop.rs`) whose exact sources are embedded in the manual; the stale
  `Minimal Example` built on retired `Request` fields moved into Quick
  Start.
- `opi-ai` (0.x): `PricingTier::input_tokens_above` now uses
  `serde_json::Number`, preserving exact integers and finite negative/fractional
  source thresholds. Integer catalog snapshots remain readable without rounding;
  Rust tier literals use integer `.into()` or `Number::from_f64` for fractions.
  `AssistantMessage` adds optional `raw_stop_reason`, preserving the original
  provider termination fact independently of normalized `stop_reason`. Old
  records read it as `None`, which is omitted on serialization; existing
  assistant-message literals add `raw_stop_reason: None`.

- `opi-ai` (0.x): `Credential::ApiKey` now owns `ApiKeyCredential`, with an
  optional key and redacted provider-scoped environment. `ResolvedAuth` adds
  its owned `env` snapshot; `AuthScheme::AnthropicFederation` identifies deferred
  workload authentication. API-key protected envelopes write version 2 and
  still read strict version 1; OAuth envelopes retain version 1. Model stores
  and dynamic sources now exchange `ModelsStoreEntry` with operation-qualified
  chat/image/classifier records and optional Unix-millisecond freshness/ETag
  metadata. Nonchat records retain model headers and input limits; image records
  retain input modalities and pricing tiers without changing base-rate billing.
  `ClassifierModelRecord::cost` is optional. Source pricing metadata can retain
  finite negative placeholders; only known selected rates produce billed cost.

- `opi-ai` (0.x): `calculate_cost_tiered` now takes the selected `WireApi`
  and returns `Option<CostBreakdown>`; unknown usage or prices stay unknown.
  `Usage` adds optional `effective_service_tier`, separate from the observed
  response tier; `ServiceTier` includes auto, scale and an unknown category.
  `ImageModelRecord::cost` and `CatalogModel::cost` are optional, distinguishing
  unknown rates from explicitly free rates. `ModelInfo` retains model headers
  and input-limit metadata; `OpenAiResponsesCompat` adds `supports_grammar`.
  Existing struct-literal callers must supply the new fields.

- `opi-ai` (0.x): `StopReason::Deferred` is a new variant and
  `AssistantMessage` gains a `deferred: Option<Box<DeferredHandle>>` field
  (task 20.50, DEL-11); exhaustive `StopReason` matches gain an arm and
  assistant-message literals gain the field. `Deferred` terminals are `Ok`
  outcomes on `ChatStream::result` exactly like `Stop`/`Length`/`ToolUse`.

- `opi-ai` (0.x): `WireApi::PiMessages` and `WireCompat::PiMessages` are
  new variants (task 20.44, DEL-11); exhaustive matches over either enum
  gain an arm.
- `opi-ai` (0.x): `WireApi::MistralConversations` and
  `WireCompat::MistralConversations` are new variants (task 20.43, DEL-11);
  exhaustive matches over either enum gain an arm.
- `opi-ai` (0.x): Bedrock Converse wire changes (task 20.42, DEL-11): the
  success tool-result images now ride real inline Converse image blocks
  instead of the `[image: ...]` text placeholder; `reasoningContent`
  blocks stream as thinking content (signature accumulation, redacted
  payloads as `RedactedThinking`) instead of being ignored; unmapped stop
  reasons fail typed carrying the raw value instead of silently mapping to
  an error stop; `model_context_window_exceeded` maps to `length`; usage
  accepts pi's `cacheWriteInputTokens` key (the established
  `cacheCreationInputTokens` stays accepted) and reports one-hour
  `cacheDetails` as `cache_write_1h_tokens`.
- `opi-ai` (0.x): `WireCompat::BedrockConverseStream` becomes a tuple
  variant carrying `BedrockConverseStreamCompat { supports_strict_tools }`
  (default false; `require` constrained tools keep failing typed before
  dispatch on this wire until a record opts in).
- `opi-ai` (0.x): `AssistantContent::Text` and `ToolCall` gain an optional
  `signature` field (task 20.40, Gemini thought signatures; serde default,
  absent in serialized history): exhaustive struct literals add the field.
- `opi-ai` (0.x): Gemini wire changes (task 20.40, P20-S40): the builtin
  catalog marks `gemini-2.5-flash`/`gemini-2.5-pro` as reasoning models, so
  a neutral explicit-off request now sends `thinkingConfig: {thinkingBudget:
  0}`; Google usage preserves an omitted reasoning child as unknown and an
  explicitly reported zero as known; unmapped finish reasons fail typed carrying
  the raw reason instead of mapping to `stop`; tool declarations ride
  `parametersJsonSchema` instead of `parameters`; success/failure function
  responses use the `output`/`error` string keys instead of `content` with
  a boolean error; Gemini-3+ ids ride function calls/responses.

- `opi-ai` (0.x): Anthropic Messages thinking wire changes (task 20.39,
  P20-S39, annex row 165): enabled thinking now always carries `display`
  (default `summarized`) on both adaptive and numeric paths; the no-budget
  default replaces the old 10000 with the source native default 1024
  (`thinkingBudgetTokens || 1024`: zero also sends 1024); the sub-1024
  typed preflight is withdrawn (clamped zero budgets flow to the wire).
- `opi-ai` (0.x): `AnthropicMessagesCompat` gains
  `supports_mid_convo_system_messages` and `supports_mid_convo_tool_changes`
  (both default false; catalog keys `supportsMidConvoSystemMessages` /
  `supportsMidConvoToolChanges`): exhaustive struct literals add the fields.
- `opi-ai` (0.x): Anthropic error tool-result images now share the turn's
  single user message with all of its tool results (text preserved, a
  `(image attached after the tool results)` placeholder text block replaces
  each image run) instead of riding a separate trailing message.

- `opi-ai` (0.x): the Azure OpenAI default api-version follows the pinned
  source `v1` (task 20.38, P20-S38), replacing `2024-06-01`; explicit
  `[providers.azure] api_version` config keeps winning. The missing-endpoint
  error message now names the resolver's base-URL wording.
- `opi-ai` (0.x): `WireApi` gains `AzureOpenAiResponses`
  (`azure-openai-responses`) and `WireCompat` gains
  `AzureOpenAiResponses(OpenAiResponsesCompat)` carrying the same responses
  compat flag surface under the azure wire identity (catalog `x-opi.compat`
  rows for the new wire parse the responses family with a `/responses`
  path default); exhaustive matches on either enum add the new arm.

- `opi-ai` (0.x): the standard OpenAI Responses wire now follows the
  pinned source defaults (task 20.37, P20-S37/ONB-03): `store: false` is
  sent unconditionally unless a catalog/profile value pins another value
  (replacing the old absent-by-default behavior), reasoning-capable
  models send `reasoning: {"effort": "none"}` on the off path
  (`github-copilot` routes skip it), and `OpenAiResponsesCompat` gains
  `supports_mid_convo_effort` (default `false`).
- `opi-ai` (0.x): `AssistantMessage` gains optional
  `provider_thinking_level: Option<String>` (task 20.37, P20-S37):
  stamped only on wires that replay the thinking level as
  mid-conversation `configuration_update` items; serde-skipped when unset,
  every struct literal adds `provider_thinking_level: None`.

- `opi-ai` (0.x): the OpenAI Chat compat config applies the single
  normalization rule (task 20.36, AIR-022/V-11).
  `OpenAiCompletionsCompat` replaces `system_role_override`, `supports_store`,
  `supports_developer_role` and the static `reasoning_effort` string with
  `store: Option<ChatStoreField>` (`Omit`/`SendFalse`),
  `instruction_role: Option<ChatInstructionRole>`
  (`Auto`/`System`/`DeveloperForReasoning`/`Developer`),
  `thinking_format: Option<ChatThinkingFormat>` (eleven declared formats) and
  optional explicit `supports_reasoning_effort`/`usage_in_stream`/
  `max_tokens_field`/`send_session_affinity_headers` values layered over a
  frozen provider-inference baseline (standard OpenAI now sends `store: false`,
  `stream_options.include_usage` and `max_completion_tokens`; legacy-compatible
  endpoints keep `max_tokens`). The retired keys fail with explicit migration
  errors at every config position (catalog `x-opi.compat`, product TOML
  profile/model compat). New compat fields cover the bridge wire behavior
  (`require_assistant_after_tool_result` now inserts the fixed assistant
  bridge), thinking-as-text replay, DeepSeek `reasoning_content` marking,
  zai `tool_stream`/vLLM `priority` extras, the three reasoning budget fields
  and `chat_template_kwargs`/`args` `$var` values. `ModelCompatOverride`
  layers `instruction_role`/`store`/`thinking_format`/`max_tokens_field`.
- `opi-ai` (0.x): `AssistantMessage` gains optional `duration_ms:
  Option<u64>` (AIR-061): monotonic adapter response-stream duration from
  activation to terminal, serde-skipped when unset; every struct literal adds
  `duration_ms: None`. `ThinkingLevelMap::nearest_supported` is public, and
  `Models::stream_simple`/`complete_simple` clamp an unsupported neutral
  thinking level to the nearest supported level (native entries retain exact
  levels and fail typed preflight instead).

- `opi-ai` (0.x): `Request` gains optional
  `telemetry_context: Option<TelemetryContext>`; request literals add `None`
  when no explicit parent is supplied. `SimpleOptions` gains
  `tool_choice: Option<SimpleToolChoice>`; struct literals add `tool_choice: None`
  or use `..Default::default()`. Codex admits neutral `Auto`/`None`; other
  currently unsupported routes reject an explicit choice before auth.
  Resource owners implement
  `SessionResourceOwner::new_namespace`; typed `ChatApi` adapters with resources
  forward leases through `session_resources`/`stream_resource_prepared`.
  Metadata-aware adapters consume the frozen complete record through the default
  `Provider::stream_model_resource_attempt` and
  `ChatApi::stream_model_resource_prepared` hooks; `ChatApi::validate_native_request`
  and `simple_options_for` retain defaults for existing custom adapters.
  `CleanupError::Incomplete { report }` carries confirmed releases and
  unconfirmed effects when cleanup cannot fully confirm completion.

- `opi-ai`: `provider::Request` gains a `callbacks:
  callbacks::RequestCallbacks` field (task 20.51): header-transform,
  payload, response and raw-event functional callbacks ride the request
  and fire inside the real direct adapters at their source-defined
  timing. `ProviderError` gains the `CallbackPhase` variant — a callback
  failure is a functional failure of the call: typed, redacted, never
  retryable, never swallowed by telemetry and never triggering a new
  call. `ProviderErrorSummary::sanitized` is now public for advanced
  error construction. Request-literal callers migrate with
  `callbacks: Default::default()`.

- `opi-ai`: the context estimator follows the 3.5 anchor contract
  (task 20.35). `estimate_context_tokens` drops the `model` parameter:
  the last applicable positive successful usage anchors the estimate
  regardless of model identity, only messages after the anchor are
  estimated, and the same-model/full-text lower bound is gone. Estimates
  use `ceil(utf16_chars / 3.5)` with 4,800 image-equivalent chars, rounded
  once per message (system tool lists round separately). `Usage` gains
  `total_tokens: Option<u64>` (independently reported total, preferred
  over the bucket sum when positive) and `service_tier: Option<ServiceTier>`;
  the former `total_tokens()` method is renamed `bucket_total_tokens()`.
  `Models::refresh` context ceilings now consume the new estimator
  arithmetic through the unchanged 4096/min-1 formula.
- `opi-ai`: `Models::refresh` becomes `refresh(allow_network: bool)`
  returning `Result<RefreshReport, ModelsError>` (task 20.33).
  `refresh(false)` performs no OAuth/network work and only restores an
  attached `ModelsStore`; `refresh(true)` fetches explicitly. Refresh is now
  per-provider: successes publish, failures retain that provider's prior
  data and surface redacted per-entry errors instead of failing the whole
  batch. All-or-nothing callers should treat `RefreshReport` entries
  individually.
- `opi-ai`: `AuthContext::process_snapshot` supplies an owned
  `AuthProcessSnapshot` (exact environment map and cwd) when a native helper
  is selected. Injected contexts default to an empty helper environment;
  hosts must explicitly supply required variables. Ambient capture stays lazy.
  ADC `ExecutableFailed::{code,message}` now carry closed
  `ProviderErrorSummary` values instead of untrusted strings.
  `OAuthCredential` and `Credential::OAuthToken` add optional `scopes`;
  struct-literal callers migrate together.
- `opi-coding-agent`: `build_harness_collection` takes a final optional
  `Arc<dyn CredentialStore>`; `CodingHarnessBuilder::credential_store`
  attaches the keychain store. The auth-aware non-interactive and RPC runner
  constructors take an optional keychain store after `auth_resolver`, before
  `extra_routes`. Production passes its existing resolver store so Models,
  credential rotation, AWS source caches and ADC owners share one queue and
  mutation barrier; store-free test assemblies pass `None`.

- `opi-ai`: ADC external-account federation and the cancelable executable
  supplier complete the `external_account` family (task 20.21.2). The new
  `auth::adc::{external,subject,executable}` modules own the pinned
  `google-auth-library@10.6.2` contracts: file and URL subject-token
  suppliers keep the raw text (no trim) or the named JSON field; an AWS
  (`aws1`) source resolves its region (`AWS_REGION` > `AWS_DEFAULT_REGION` >
  `region_url` with the availability-zone suffix stripped) and security
  credentials (environment first, else the security-credentials endpoint
  with an optional IMDSv2 session-token PUT at the fixed 300 s TTL, one
  token fetch per supplier leg, every IMDS leg a single send), then signs a
  serialized `GetCallerIdentity` request with SigV4 through the existing
  signing primitive — `x-goog-cloud-target-resource` is merged after
  signing, exactly as upstream — and percent-encodes the
  `JSON.stringify({url, method, headers})` subject token. The certificate
  source resolves its configuration (`certificate_config_location` >
  `GOOGLE_API_CERTIFICATE_CONFIG` > the well-known gcloud path), validates
  leaf and chain through the rustls parser, builds the base64(DER) JSON
  chain subject token with the exact leaf-order rules (absent → prepended,
  first → kept, later → typed failure), and exchanges over a source-scoped
  mTLS transport built from the frozen TLS profile (the identity never
  rides another leg). The executable (pluggable) supplier checks the
  `GOOGLE_EXTERNAL_ACCOUNT_ALLOW_EXECUTABLES=1` admission gate **before**
  the output-file cache and any spawn, parses the command into a direct
  quote-aware argv (never a shell), merges stdout and stderr, validates the
  response schema (version/success/token-type URN with its token field,
  `expiration_time` required when an output file is configured, expiry),
  treats a malformed cache as terminal but a stale one as a miss, reuses the
  crate's private bounded-process mechanism (process group, kill-on-drop,
  tree reaping on timeout), and never rewrites the `output_file`. The
  RFC8693 STS exchange (`grant_type=token-exchange`, audience,
  `subject_token`, `subject_token_type`, scope) defaults to
  `https://sts.{universe_domain}/v1/token`, uses HTTP Basic when the file
  carries `client_id`/`client_secret`, and flows a workforce
  `workforce_pool_user_project` into the non-standard `options` form field
  only without client authentication; exchanges run the fixed Gaxios retry
  budget. A new `AdcCredentialSource::ExternalAccount` provenance variant
  classifies the supplier non-secretly;
  impersonation composes through the recursive source/target owner.
  The ADC owner's canonical-endpoint override now also pins the external
  exchange URL. Acceptance test sources gain a platform-neutral executable
  helper through `test_support::portable_executable_script`.
- `opi-ai`: Vertex generation auth gains the pinned ADC native families and
  the dual-mode wire (task 20.21.1). The new `auth::adc` module owns the
  exact contracts from the locked `google-auth-library@10.6.2` /
  `gaxios@7.1.4` / `@google/genai@2.21.0` / `gcp-metadata@8.1.2` sources:
  a `service_account` file signs an RS256 JWT (alg-only header; claims
  `iss`/`scope`/`aud`/`iat`/`exp = iat+3600`) through the RDF-23 crypto
  owner and exchanges it at the canonical
  `https://oauth2.googleapis.com/token` endpoint (the file's `token_uri`
  never becomes a credential destination); an `authorized_user` file
  refreshes with all four form fields and keeps `quota_project_id`
  separately (it rides the generation request as `x-goog-user-project`); an
  `external_account_authorized_user` file refreshes with HTTP Basic client
  auth at its validated `token_url` or the default
  `https://sts.{universe_domain}/v1/oauthtoken` endpoint, and a returned
  refresh token replaces the owner's copy in place (the ADC file is never
  rewritten by default). The explicitly selected direct metadata source
  serves `GET .../service-accounts/default/token` with the
  `Metadata-Flavor: Google` exchange in both directions, the frozen
  residency timeout (`true` → none, otherwise 3000 ms), and the measured
  single-send timeout. Token exchanges run the fixed Gaxios call
  configuration (POST admitted, 3 extra retries, 2 no-response retries,
  statuses 1xx/408/429/5xx, 100/500/1500 ms delays) through an injectable
  transport with token POST redirects terminal and source GETs bounded to
  twenty redirects; they never enter the generation retry owner. File selection keeps the source's precedence: an explicit
  `GOOGLE_APPLICATION_CREDENTIALS` path (scoped or ambient) is terminal on
  read/parse failure, the portable well-known file
  (`%APPDATA%\gcloud\...` / `$HOME/.config/gcloud/...`) means not
  configured when absent — missing configured credentials never trigger
  metadata, BIOS, or gcloud discovery. External accounts and impersonation
  use their separately owned supplier and source/target mechanisms. A new
  `AuthProvenanceSource::Adc` variant carries the non-secret source facts
  (family, quota project, rotation). `VertexProvider` now serves both
  pinned wire modes: API-key Express (`x-goog-api-key` on the
  project/location-less `aiplatform.googleapis.com/v1/publishers/...`
  collection URL, `location = "global"` pins the shared domain on the
  ADC route) and Bearer ADC with project/location; the provider factory's
  vertex route resolves store credential/`GOOGLE_CLOUD_API_KEY` (Express) →
  configured access-token env (Bearer) → ADC chain → typed
  `CredentialNeeded`, with the Gemini key never reused for Vertex and a
  half-configured project/location pair a typed construction error.
- `opi-ai`: Bedrock generation sends retry through the pinned AWS Smithy V2
  modes (task 20.20). The new `bedrock::retry` module owns the exact
  algorithm from the locked `@smithy/core@3.33.3` retry middleware and the
  Bedrock error model: `standard` and `adaptive` are the two known modes
  (an unknown mode is a typed configuration failure, never `Unsupported`
  and never silently standard), the Smithy2026 constants are an explicit
  frozen branch, and scoped `AWS_RETRY_MODE`/`retry_mode` and
  `AWS_MAX_ATTEMPTS`/`max_attempts` inputs resolve with the source's
  environment-over-profile-over-default precedence (shared-config entries
  read through a minimal profile-section reader) without sampling ambient
  process state. Failure classification is exact through a new structured
  `ProviderError::ServiceFault` variant (HTTP status plus the
  `x-amzn-errortype` wire name): 429 and throttling names are THROTTLING,
  the modeled `$retryable` trait (`ModelNotReadyException`), transient
  names, transport timeouts/network failures, and statuses 500/502/503/504
  are TRANSIENT, bare other 5xx are SERVER_ERROR, and client faults
  including `ModelTimeoutException` on 408 are CLIENT_ERROR — no blanket
  408/409/501/505 retry and no helper 60-second hint cap on Smithy. The
  acquisition owner (`AcquisitionPolicy` gains a `RetrySourcePolicy`:
  helper rules stay the default; `Smithy(SmithyRetryConfig)` applies the
  AWS family) enforces the source's admission pair (retryable class,
  `count+1 < max_attempts`, per-call quota capacity) with the per-logical-
  call quota starting at 500 (costs 5/10, or 5/14 with 2026 constants;
  success refunds the spent cost or +1), full-jitter backoff
  `floor(u * min(base * 2^count, 20s))` on the 100/500ms (50/1000ms in
  2026) bases, server hints (`Retry-After` seconds/HTTP-dates/` GMT, N`
  and `x-amz-retry-after` milliseconds) bounded by
  `max(delay, min(hint, delay + 5s))`, and adaptive mode's CUBIC
  client-side rate limiter (beta 0.7/scale 0.4/smooth 0.8 EWMA over
  half-second buckets, multiplicative cut on throttling, cubic recovery,
  send-token waits folded into the same cancellable readiness deadline;
  injectable `SmithyClock` for deterministic tests). Ordinary
  `Models::prepare_call` now freezes the provider's protocol default
  (`Provider::acquisition_policy_default`; Bedrock = its frozen Smithy
  config, helper/Codex wires keep zero extras), and each Bedrock send
  stamps the wire `amz-sdk-invocation-id` (one uuid v4 per logical call,
  threaded through the new `Provider::stream_attempt` dispatch with its
  `AttemptContext`) and `amz-sdk-request: attempt=N; max=M` headers while
  re-signing SigV4 with a fresh timestamp over the byte-identical frozen
  payload and credential. Auth-source exchanges never draw from the
  generation quota; the two send families stay separately counted. The
  `uuid` workspace dependency gains the `v4` feature. Migrated callers:
  `opi-agent` diagnostics/summaries and the interactive auth presenter map
  the new `ServiceFault` variant (same provider-side category and
  presentation as `ProviderSide`).
- `opi-ai`: the local AWS chain closes the SSO and login-session families
  and the complete source cache (task 20.19.3). The chain order is now
  Env -> explicit SSO -> INI (assume-role -> static -> web identity ->
  process -> SSO -> login_session) -> environment web identity -> ECS or
  IMDS: any explicit-init SSO field (scoped start URL/account/region/role/
  session) selects the SSO segment — a partially given selection is a typed
  terminal (`IncompleteSsoConfiguration`), never "unconfigured" — and a
  profile's SSO fields or `login_session` key select inside the INI family.
  SSO resolves modern `sso_session` profiles (config-file `[sso-session]`
  sections, explicit fields win) and legacy inline profiles through the
  standard token cache (SHA-1 file names; modern keys are session names,
  legacy keys are upper-cased start URLs): a near-expiry token is returned
  as-is with a single throttled background refresh (5-minute window,
  per-session 30-second throttle) registered into the shared credential
  mutation owner; an expired modern token refreshes actively through SSO
  OIDC `POST /token` (denials typed with their standard code, including
  slowdown/throttle), a failed refresh with an unexpired token keeps the
  old token, a successful refresh writes the standard cache file with the
  write failure ignored, and legacy tokens never refresh (re-login). The
  federation exchange is `GET /federation/credentials` with the
  `x-amz-sso_bearer_token` header and epoch-millisecond expiry; missing
  values, invalid payloads, and denials are typed terminal failures that
  never discover another identity (zero generation sends).
  `login_session` reads the standard login cache (SHA-256 of the trimmed
  session name), refreshes through SignIn `POST /v1/token` with an owned
  ES256 DPoP proof (RFC 9449 `typ:dpop+JWT` header with JWK `x`/`y`,
  `jti`/`iat`/`htu`/`htm` payload, deterministic RFC 6979 signature,
  injected clock/`jti` source), extracts the three AWS credentials from
  the access token, keeps account provenance frozen to the cache, and
  writes rotated refresh tokens back (the in-memory cache keeps the
  rotation even when the file write fails, so the unique token is never
  lost): a bad/missing selected cache, DPoP failure, and the terminal
  AccessDenied family stop; other refresh failures keep a still-valid
  token and, once expired, continue the chain. The new injectable
  `auth::aws::cache::AwsSourceCache` owns single-flight active lookups,
  the refresh windows and throttle, per-key revisions (sub-token caches
  and outer temporary credentials keep distinct identities and receipts),
  explicit invalidation, and mutation-owner registration (awaitable via
  the queue barrier after the caller leaves); `AwsLocalChainInput` gains
  `source_cache`, `clock`, `dpop_jti`, the SSO/login cache directories,
  and SSO/OIDC/SignIn endpoint overrides (`ScopedAwsEnv` gains the five
  explicit-init SSO fields); `AwsCredentialSource` adds `Sso`/
  `LoginSession` provenance (mirrored by `opi-agent` evidence facts);
  `credential_process` budgets become the injectable
  `CredentialProcessPolicy` (AIR-023 declared opi policy). Dependencies:
  `p256 =0.13.2` (ES256 DPoP/JWK) and `sha1 =0.10.6` (standard SSO cache
  file-name hash only) per the pinned table.
- `opi-ai`: the local AWS chain extends to the STS/web-identity and
  remote source families (task 20.19.2). The shared-config INI family follows
  the verified `resolveProfileData` order (assume-role/source-profile →
  static → web-identity → `credential_process`); `role_arn` requires exactly
  one of `source_profile`/`credential_source` (ambiguous or incomplete role
  configuration, profile cycles, and unsupported `credential_source` values
  are continue-eligible), nested source profiles resolve statically-first
  with cycle detection, `credential_source` routes Environment/EcsContainer/
  Ec2InstanceMetadata providers, and a profile-required MFA without an
  injected presenter is a typed terminal failure (`TerminalReason`,
  `MfaPresenterMissing`). STS `AssumeRole` exchanges POST the pinned
  `aws-smithy-query`-encoded form signed with the *source* credentials
  (service `sts`, regional endpoint from `auth::aws::endpoint`, explicit
  override wins, China partition `.cn` suffix); responses pass an owned
  `xmlparser` lexical/structure/DTD precheck that fully collects text/CDATA
  segments and the pinned `aws-smithy-xml` reader gates structure, with
  expiration/`AssumedRoleUser`/subject/audience/provider metadata retained
  (account id from the ARN) and `ErrorResponse` denials kept as typed,
  secret-free status/code/request-id. Web identity reads the selected token
  file (`web_identity_token_file` profile key or the environment segment
  `AWS_WEB_IDENTITY_TOKEN_FILE`/`AWS_ROLE_ARN`/`AWS_ROLE_SESSION_NAME`) and
  exchanges anonymously — never source SigV4; unreadable files, STS denials,
  and invalid responses are terminal. The remote segment resolves ECS
  (relative URI preferred over full URI, token file preferred over direct
  token, https-any-host/loopback/link-local http host rules, 1s per attempt,
  at most four modern sends with a 1000ms delay, then exactly one legacy
  send; a selected ECS segment never falls through to IMDS and an invalid
  endpoint is terminal) and IMDS (v2 token first, token 400 stops the
  request, timeout/403/404/405 enable v1 mode for subsequent legs with other
  token failures one-shot v1 fallback, `AWS_EC2_METADATA_DISABLED` gates the
  default remote branch and the v1 fallback, 401 on the role/credential legs
  clears the token state and refetches, 1s/zero retries per leg, IPv4/IPv6
  endpoint mode with override). All STS/web-identity/ECS/IMDS sends run on
  the new injectable `auth::aws::source_http` transport (controlled client,
  redirects refused as typed errors) so auth-source sends stay separate from
  generation attempts; resolution cancellation releases in-flight exchanges.
  `AwsLocalChainInput` gains required `transport`, `mfa`, and `sts_endpoint`
  fields; `ScopedAwsEnv` gains the web-identity, ECS, and IMDS keys;
  `AwsCredentialSource` adds `AssumeRole`/`WebIdentity`/`EcsContainer`/
  `Ec2InstanceMetadata` provenance (mirrored by `opi-agent` evidence facts).
- `opi-ai`: the local AWS credential chain moved from
  `bedrock::credentials` to `auth::aws::local`/`auth::aws::process` and now
  follows the verified locked-Node source rules (task 20.19.1): an explicit
  or scoped `AWS_PROFILE` skips the environment pair (an ambient profile
  alone does not beat a complete pair), shared-config static credentials
  overlay the credentials and config files per field (credentials file wins
  per key), a partial ambient pair, unreadable profile files, and every
  `credential_process` discovery failure are continue-eligible, and only a
  partial scoped/typed pair fails closed (`IncompleteSource`).
  `credential_process` runs with a frozen command/cwd/env through the
  injectable `CredentialProcessRunner` seam (default spawn keeps the existing
  10s/64 KiB opi budgets and process-tree cleanup) and validates the actual
  `Version` plus RFC 3339 `Expiration`; expiry is retained on the resolved
  credentials. `CredentialResolutionError` reduces to `Exhausted` /
  `IncompleteSource`, and `bedrock::credentials::{resolve_auth,
  resolve_credentials, CredentialResolutionInput, read_profile,
  credentials_from_env}` were removed with the module.
- `opi-ai`: `AwsSigV4Credentials` gains a non-secret `expires_at` field plus
  `new`/`with_session_token`/`with_expires_at` constructors; struct-literal
  callers migrate together. `AwsCredentialSource` adds `ScopedEnvironment`
  and `ProxyPlaceholder` provenance (mirrored by `opi-agent` evidence facts).
- `opi-ai`: `AuthResolutionOverrides` gains typed mutually exclusive
  `bearer` and `aws_sigv4` per-call overrides; more than one explicit
  override fails with the new `ModelsError::ConflictingAuthOverrides`
  before any source access.
- `opi-ai`: the Bedrock adapter attaches prepared Bearer authentication
  (`Authorization: Bearer`, no SigV4 headers; endpoint from provider config
  or the resolver-supplied base URL) next to SigV4, never both. The explicit
  no-real-credential proxy-placeholder mode (`bedrock::proxy_placeholder_auth`)
  signs with fixed nonsecret placeholder SigV4 on a caller-supplied
  nonstandard endpoint, requires the dedicated HTTP/1.1 placeholder client
  (`BedrockProvider::with_placeholder_client`), reads zero credential
  sources, and standard AWS endpoints reject it.
- `opi-coding-agent`: the Bedrock route resolver now runs the migrated
  `auth::aws` chain; an explicitly configured `providers.bedrock.profile`
  skips ambient environment credentials and a partial ambient pair continues
  to the shared-config profile instead of failing closed.
- `opi-ai`: `CredentialMutationOp` now distinguishes `Write`, `Delete`, and
  `Update(CredentialUpdateOp)`; custom callbacks wrap `Update`. Unconditional
  login replacement and logout can repair corrupt entries without decoding
  their previous value.
- `opi-ai`: default `HttpClient` TLS now uses the same fixed instance AWS-LC
  profile and packaged Node roots as explicit `TlsOptions::default`, without
  inheriting the host's global provider or backend default roots.
- `opi-ai`: schema regex and URL/IDNA data now follow the supported crates.io
  dependency profile instead of a fixed Unicode 16 fork (RDF-21). Newly assigned
  letters, script names and domain mappings can change schema admission,
  matching and normalization, including when saved declarations are recompiled.
  TypeBox algorithms, typed failures and tool authorization remain unchanged.
- `opi-ai`: `SchemaError` adds `InvalidSchema { path }` for root null and
  root/referenced number/string schema nodes that the pinned compiler cannot
  build. A reference resolving to null retains the source's false-target fallback;
  JSON arrays keep the source's inert behavior.
- `opi-ai`: `normalize_context` returns `Result<TranscriptContext, TranscriptError>`;
  initial shorthand, embedded system declarations and deserialization reject
  invalid declaration batches before dispatch. `ToolResultMessage` adds a
  serde-defaulted `not_executed` reason written by trusted execution boundaries;
  historical absence remains unknown, never inferred from error text.
- `opi-ai`: `HttpClientBuilder::build` now returns
  `Result<HttpClient, HttpClientBuildError>` instead of `reqwest::Error`;
  rejected TLS configuration is typed and redacted (`TlsConfigError`,
  `TlsKeyError`) while backend construction failures wrap `reqwest::Error`.
  All current callers migrate together with no behavior change on the
  non-TLS paths.
- `opi-ai`: the prepared call is the one acquisition-retry owner (AIR-007 /
  task 20.4). `Models::prepare_call_with_policy` freezes an `AcquisitionPolicy`
  per logical call (`prepare_call` keeps the protocol default); `start_attempt`
  now refuses dispatch with new typed `ModelsError::RetryNotReady` before an
  admitted retry's readiness deadline and with `ModelsError::CallSealed`
  (`AcquisitionSeal`: `PostHandoff`/`UnknownDispatch`/`Exhausted`/`Finished`)
  once the call is sealed. Successful response handoff seals acquisition
  retry, including accepted Chat responses that fail before the first SSE
  event; a dispatched attempt dropped without an observed outcome seals as an
  unknown effect, and success no longer re-opens a prepared call — callers
  that re-dispatched a finished call (including tests) must schedule the
  admitted retries through `retry_admission()` or re-prepare. `opi-agent`
  mirrors the refusals as `AgentError::RetryNotReady`/`AgentError::CallSealed`.
- `opi-agent`: the agent loop no longer schedules acquisition retries itself.
  The product retry count becomes the per-call SDK policy and the loop
  schedules only the attempts the prepared call admits, waiting cancellably on
  the SDK-computed delay (server hints first, otherwise the source family
  backoff `min(500ms * 2^i, 8000ms)` with `[0.75, 1.0)` jitter instead of the
  old product 1000ms/no-jitter/60s clamp). `AutoRetryStart`/`AutoRetryEnd`
  events, retry diagnostics and Retry evidence records keep their shapes;
  the summary `ProductCompactionHooks::prepare` new-call recovery and
  `PreparationEvidence::run_provider_call` (single-send, zero extra sends)
  are unchanged.
- `opi-ai`: the shared `HttpClient` no longer auto-detects ambient
  `HTTP_PROXY`/`HTTPS_PROXY`/`NO_PROXY` environment variables (RDF-22):
  proxy use is explicit-only through `ProxyConfig`, whose scoped `no_proxy`
  list decides bypassing. Consumers that want environment-derived proxies
  resolve them with `proxy_from_env` and pass an explicit config, as the
  coding agent already does.

- `opi-ai`: `ProviderCollection` is replaced by a single `Models` driver
  (`provider_collection` module renamed to `models`; `CollectionError` is
  now `ModelsError`). All current callers migrate together: `register`,
  `register_route`, `prepare_call`, `refresh`, catalog/auth reads and the
  prepared-call lifecycle keep their contracts under the new owner, with no
  parallel facade or router.
- `opi-ai`: one final `Result` per logical provider call. `CompletedRequest`
  is removed; `drain_to_completion` returns
  `Result<AssistantMessage, CallFailure>`, sharing the new
  `ChatStream::result` seam (`Stream<Item = Result<AssistantStreamEvent,
  CallFailure>>`). The first delivered terminal is absorbed into permanent
  EOF, a stream that ends without a terminal yields exactly one typed
  protocol failure, and every failure carries the preserved partial
  message (content, provenance, diagnostics, known usage) plus a typed
  redacted cause; cancellation maps to the `Aborted` stop reason and
  setup failures have no generated content with unknown usage.
- `opi-ai`: current providers stop the dual terminal format. In-band
  provider error events (`anthropic`, `openai_chat`, `openai_responses`
  family, `gemini`, `bedrock`) now terminate the stream with a bounded typed
  `StreamError` cause instead of an `AssistantStreamEvent::Error` terminal;
  the variant is retained read-only for deserializing historical saved
  events and is converted at the `ChatStream` seam.
- `opi-agent`: the agent loop and summary `run_provider_call` consume the
  public `ChatStream` seam. Failures classify retries through the typed
  cause as before, and failed calls now preserve the canonical partial in
  the turn's events and Agent state (in-band terminals unchanged; mid-stream
  failures previously lost the partial). Premature EOF surfaces as the
  typed stream-failure error instead of `AgentError::ProviderProtocol`.

### Fixed

- `opi-ai`: Chat routing and Chat/Responses strict/grammar, cache and affinity
  controls reach wire payloads, headers and replay, preserving false, zero,
  empty and present-null values with per-wire defaults and Azure/Codex exceptions.
  Declared finishless Chat streams finalize without inventing a raw stop reason.
- `opi-ai`: strict schema conversion rejects malformed `required` values and
  unknown required property names without changing the trusted schema. For
  `Require` declarations retained by the selected wire projection, malformed
  `required` shapes fail before authentication; existing `Prefer` fallback is preserved.
- `opi-ai`: generation HTTP requests use the source-qualified default limit of
  20 redirects, preserving request bodies and same-origin authentication.
  Explicit caller limits and auth-source, OAuth and WebSocket policies are unchanged.
- `opi-ai`: provider inventory, removal, clearing and operation selectors cover
  actual registrations, with availability checked per registration. Selected
  refreshes restore only matching APIs, forward force to selected sources and
  merge publication through the store ACK while retaining unselected records.
  Removal leaves shared stores and already captured calls intact.
- `opi-ai`: Chat, image, classifier and deferred registrations accept exact
  arbitrary `ApiId` keys, including empty and whitespace-only strings, without
  authentication or network access. Provider-ID validation remains unchanged.
- `opi-ai`: a known positive independently reported usage total anchors the
  context estimate even when parent buckets are unknown. An absent or zero
  total falls back only to a known positive parent-bucket sum.
- `opi-ai`: projection facts and the latest canonical partial survive stream
  failures and premature EOF, and projection wrappers absorb the first terminal.
  Strict temporal rejection retains its typed cause through the existing
  redacted error carrier; logical-call cancellation stays distinct from an
  authentication source reporting cancellation.
- `opi-ai`: Chat, ordinary Responses and Azure Responses measure duration from
  their first response-body poll, including an initial pending body wait while
  excluding authentication, headers, response callbacks and unpolled time.
- `opi-ai`: ADC AWS region decoding removes the availability-zone suffix at a
  UTF-8 character boundary, preventing non-ASCII metadata from panicking.
  Empty regions and supplementary suffixes that cannot retain the source's
  UTF-16 result fail typed before token exchange; region precedence is unchanged.
- `opi-ai`: ADC metadata and native token exchanges check expiration arithmetic;
  an unrepresentable positive lifetime returns `InvalidTokenResponse` instead
  of panicking. Missing and nonpositive lifetimes retain unknown expiration.
- `opi-ai`: schema-owned admission, reference lookup, normalization,
  subvalidator cloning and destruction use iterative work stacks, preserving
  resolver order and normalization semantics for deeply nested schemas through
  last-owner release and compile-error cleanup.
- `opi-ai`: the context-overflow 99-percent comparison uses widened arithmetic
  so large `u64` context windows cannot panic or wrap, while preserving known
  usage requirements and ordinary threshold behavior.
- `opi-ai`: deferred cancellation validates the handle identity, then honors
  caller cancellation before and during authentication so a cancelled wait
  cannot dispatch a late backend cancellation request.
- `opi-ai`: Chat, Responses and Gemini SSE parsers preserve CRLF boundaries
  split across network chunks and join multiline data fields before parsing,
  retaining complete event framing under fragmented delivery.
- `opi-ai`: cleartext ECS credential endpoints require a parsed loopback or
  link-local IP address or exact `localhost`; lookalike DNS hosts are rejected
  before token-file access or source requests. HTTPS endpoints, relative-URI
  precedence and existing retry budgets are preserved.
- `opi-ai`: Bedrock validates each complete 12-byte event-stream prelude
  before waiting for its declared payload, rejecting invalid CRCs, undersized
  frames and impossible header lengths. Valid incomplete frames remain buffered,
  and complete frames retain message-CRC validation.
- `opi-ai`: image (OpenRouter Images) and classifier (System One) payload
  observers now run inside the caller's frozen `TelemetryContext` section,
  so their observation routes to the frozen parent/Dispatch exactly like
  the wire attempt instead of poll-time ambient dispatch (task 20.55,
  DEL-12).
- `opi-ai`: Mistral requests apply canonical replay projection before wire
  serialization, dropping foreign or unknown-source thinking while preserving
  raw string tool arguments and paired tool-result IDs. Google and Vertex part
  signatures require the exact provider/API/model source, and Chat preserves
  opaque-only `reasoning_details` on same-identity replay.
- `opi-ai`: Google and Vertex thought signatures reject misplaced or excess
  base64 padding and retain source-accepted unused-bit forms such as `AB==`.
  Bedrock redacted replay accepts omitted padding, ASCII whitespace and nonzero
  unused bits, while omitting malformed or empty decoded payloads.
- `opi-ai`: Google and Vertex preserve independent usage fields and reported
  totals, including usage-only frames after a finish reason. Missing child
  subsets remain `None`; measured duration starts at adapter response-stream
  activation, excludes Models auth/loading and survives terminal failures.
  Pricing tiers round-trip through builtin, dynamic and personalized catalogs
  and all three stored operations. Selection starts at `-1`, admits only a
  strictly higher threshold strictly exceeded by input, keeps the first equal
  tier and preserves integer thresholds and token comparisons above `2^53`.
- `opi-ai`: classifier JSON syntax, callback and answer-validation failures
  no longer trigger low-level acquisition retries. Source-qualified network,
  timeout and eligible HTTP retries remain; Decisions 504 stays non-retryable.
  A failed Decisions answer clears the whole answer set while retaining known
  usage and cost. Builtin display names use Amazon Bedrock, Anthropic and
  Google Vertex AI, independently of their credential labels.
- `opi-ai`: real Anthropic and Codex producer replay regressions cover terminal
  capture, serialization/reload and subsequent same-identity, changed-model and
  foreign-wire requests while preserving canonical opaque data.

- `opi-ai`: stored scoped configuration now reaches Cloudflare account/gateway,
  Azure routing/deployment, Vertex ADC and AWS native sources without changing
  accounts. Anthropic workload federation performs the selected endpoint's
  token exchange lazily, shares source-qualified refresh state without restarting
  old token expiry after a cancelled wait, and invalidates on 401 without
  implicitly resending the generation. Google thinking config
  now uses REST `generationConfig`; Vertex custom model API bases follow the
  pinned collection/version rules. Model-store restoration preserves all three
  operations, freshness metadata and authoritative empty catalogs.

- `opi-ai`: complete model records and native effort reach the selected
  adapters. Image/classifier calls own their registrations before first poll;
  provider-qualified registrations no longer replace another provider's auth.
  Nonstream callbacks use independent call payloads and cannot trigger an
  acquisition retry after response handoff. Malformed usage retains known
  counts without claiming a reliable zero or total. Google/Vertex capture and
  replay retain canonical identities and part-level tool signatures; Pi
  Messages preserves its context envelope, authoritative tool arguments, simple
  passthrough options and partial failures; Mistral retains partial output and source cache precedence.
- `opi-ai`: builtin routes preserve source-qualified stored-key/OAuth selection,
  lazy Azure/Vertex/AWS endpoint resolution and all operation records. Azure
  freezes its endpoint and deployment before acquisition attempts. Native
  calls reject invalid model input before authentication; cancelled image and
  classifier calls release pending authentication waits. Model
  header overrides retain removal semantics and protected authentication.
  Explicit Chat/Responses model API bases preserve their version path without
  appending it twice; constructor-only root endpoints retain their existing path.
  Shared catalog-store revisions observe explicit deletion and source
  invalidation before refresh; persisted tombstones fence late publications.
  `catalog::personalize_openrouter_catalog` explicitly returns account-local
  chat limits/prices while preserving nonchat records, with no automatic
  factory fetch or publication.
- `opi-coding-agent`: canonical reference migration checks target conflicts
  before changing defaults. Credential account copies serialize with normal
  mutations, acknowledge the complete target before source cleanup, and can
  resume interrupted same-account writes without taking over ambiguous markers.


- `opi-ai`: Codex seals fallback/reconnect on valid provider events even before
  public output, preserves safe fallback diagnostics, disables idle/connect
  timers for explicit zero, and evicts idle/aged pooled connections on acquire.
  `CodexOptions::websocket_connect_timeout` selects the independent connection
  budget (omitted: 15 seconds) without adding a wire field. Chat/Responses keep
  safe quota facts for explicit whole-call retry while preserving acquisition
  status/hints. Chat/Responses/Codex reject malformed callback payload shapes
  before dispatch while retaining legal extensions. Store-free catalog refresh fences cancelled late results,
  grammar keys use literal property names, and multi-block mock partials retain
  only the content emitted at each event.

- `opi-ai`: ordinary and advanced acquisition share one bounded owner, including
  forwarded Anthropic strict variants; successful response headers seal retries
  before callbacks or body errors. Async callbacks preserve ordered raw
  observation, per-call header freezes and per-variant payload freezes; Google
  raw callbacks are supported. Codex telemetry retains the source span's
  subscriber, and cleanup registers before auth, isolates instance namespaces,
  cancels pending producers and awaits all local releases without repolling a
  dormant caller Future.
- `opi-ai`: native Chat/Responses calls retain native token-limit omission and
  explicit values; simple preparation estimates the canonical System chronology
  after usage anchors. Catalog source failures retain acknowledged revisions;
  store round-trips retain sampling/strict metadata and zero, duplicate or
  unordered pricing tiers. Final JSON parsing repairs complete object strings
  with control characters as well as invalid escapes; grammar deltas accumulate
  and terminal-only custom-tool input is retained.
- `opi-ai`: `CodexChatApi` exposes the real Codex adapter through
  `ChatApi<CodexOptions>`, with typed reasoning effort/summary, service tier,
  text verbosity and tool choice. Native defaults remain distinct from simple
  thinking mapping, and explicit service-tier null survives on the wire.

- SDK OAuth routes resolve acknowledged stored identities through the same
  flow used for login and refresh before dispatch. Initial identity selection
  happens inside the store slot, including fresh-token reads. Replacing the
  attached store or upserting a helper-bound flow rebinds new OAuth calls;
  previously captured calls keep their frozen binding. The product `/login`
  helper delegates to SDK `login_to_store`; presenter text/progress forwarding
  and token-plus-metadata ACK ordering are shared. Issued ChatGPT client ids,
  validated scopes, Copilot enterprise domains and account model allowlists
  survive host-envelope save/reopen. `Some([])` account model metadata stays
  authoritative; `None` preserves the registered catalog.
- OAuth token fields reject newly empty access/refresh values according to
  each flow. Anthropic/xAI omitted-refresh tolerance and OpenRouter's
  permanent-key empty refresh slot remain unchanged. Source duration overflow
  returns typed failures, and Copilot applies its existing source limits
  before duration conversion. Anthropic copy-code waiting/presentation shares
  the configured login budget and cancellation waiter; its double-bind failure
  retains manual fallback. Radius browser login accepts callbacks only.
- Native AWS/ADC helpers execute the selected command with frozen cwd and
  environment, without rediscovering ambient variables. ADC admission precedes
  cache reads; a valid output cache does not capture native process state.
  Helper timeout/drop cleanup drains and reaps owned processes; public ADC
  errors never retain arbitrary helper code/message/output. Existing AWS
  10-second/64-KiB host-policy defaults remain unchanged.
  Windows helpers start suspended, join an owned kill-on-close Job, then
  resume; descendant cleanup continues after the helper parent exits. A
  private Windows owner isolates the necessary unsafe OS calls using
  `OwnedHandle`, without unsafe `Send`. The Windows-only `windows-sys` 0.61.2
  dependency adds `Win32_Security` to existing Job/ToolHelp/Threading features;
  no new package version or build script is introduced. Unix process groups
  remain unchanged.
- Nested AWS role profiles resolve their complete leaf chain before applying
  roles from inside out, retaining static-pair precedence. SSO source caches
  isolate region/account/role/session, endpoints and source revisions. Owned
  rotations survive waiter cancellation through source-memory publication,
  best-effort file save and receipt; a receipt does not assert durable standard
  cache-file success. Invalidation fences late memory/file writes without
  claiming remote rollback. Missing rotation ownership fails typed before an
  exchange, and expired access is not dispatched.
- ADC owners share the store mutation queue, coalesce concurrent refreshes,
  retain started rotating exchanges after waiter cancellation, and fence late
  token plus refresh-material publication after source replacement. Actual
  short TTLs remain usable for the current call while valid. Token POSTs
  refuse redirects; source GETs follow at most twenty redirects with sensitive
  headers removed across origins.
- Acquisition checks and attempt-lease reservation share one synchronized
  owner. A terminal attempt remains at EOF and cannot later change another
  attempt's retry state; accepted Chat response body failures never admit a
  replay, including a timeout before the first SSE event.
- Classic WebSocket upgrades use HTTP/1.1 with the same frozen TLS identity,
  trust and proxy inputs, while ordinary HTTP requests keep their H2 profile.
  The SDK dependency notices identify the actual Zlib licenses
  and compression backend selected by each supported feature union.
- HTTP trailer acceptance fixtures now consume the complete HTTP tail before
  success, exposing the upstream field-count/byte boundaries and malformed or
  truncated tails instead of ending at an earlier SSE terminal.
- Prepared attempts defer adapter construction and dispatch until first poll;
  dropping an unused lease sends nothing, while dropping a dispatched attempt
  seals its unknown effect. SDK HTTP clients disable backend protocol-NACK
  retries so every additional send requires SDK admission.
- Credential queues retain off-runtime submissions in FIFO order until an
  ACK poll or normal submission hands them to a host Tokio runtime. Started
  work survives waiter cancellation; barriers only observe and wait. Failed
  WebSocket Close now discards retained receive events and remains at EOF.
- Package-consumer checks bind native provenance to each consumer's actual
  Cargo units, verify registry/source identities, and reject unreviewed native
  replacement inputs instead of accepting unrelated shared-cache archives.
- Completion futures freeze owned calls before polling and remain `Send +
  'static` after the driver is changed or dropped; tool-registry installation
  freezes validation against the final declaration rather than assembly cache.
- Error/Aborted terminals deliver one failure; errors, cancellation and premature
  EOF retain canonical raw/opaque content and known usage. Receiver drop releases
  current adapters' connect/read/backpressure work.
- Replay validates association before omitting failure pairs, rejects conflicting
  effect facts, retains unknown effects and prevents legalized wire-ID collisions.
  Missing results in omitted Error/Aborted turns retain their unknown effects in
  sanitized ordinary user context, including after serialization.
- Product session turn and compaction writes preserve trusted `not_executed`
  facts through save/resume; historical absence remains unknown. Malformed
  proposals rejected before execution remain replayable after restoration.
- Public schema checks use upstream `regress` and `url` with an explicitly
  recorded data-profile difference. Annotation data is not a validation schema; referenced targets keep
  their guards. Exact native integers retain equality, bounds and uniqueness.
  Registry publication and final package-consumer acceptance remain open.
  Malformed properties/patternProperties maps are inert as a whole, while
  schemas referenced inside const/enum retain evaluated-key/index tracking.
  Optional nulls survive independent sub-schema compilation failures, while
  failed sub-compiles remain non-matching union arms.
  Regex admission follows source keyword order and freezes reference targets
  in the compiled plan, including resource effects from absent instance fields.
  Relative-reference recursion returns `NoProgress` only after proving a
  repeated compiler prefix's URL decisions and function-cache membership remain
  invariant; later regex targets and legal resource resets are still visited.
  Resource identities use node identity, and resolved targets retain the
  source's overwrite-and-single-exit behavior.
- Package consumer checks reject any set `ICU4X_DATA_DIR`, including the empty
  string, before creating scratch data or invoking a subprocess. This enforces
  the upstream ICU bundled-data precondition for dependency-profile verification.

- `opi-ai`: source-configuration checks moved off the IO-free descriptor.
  `AuthDescriptor::resolve`, `AuthStatus`, and the synchronous
  `Models::auth_status` (which read `std::env` directly) are removed in favor
  of the on-demand management results below; `ModelsError` is now
  `#[non_exhaustive]`.

- `opi-ai`: additive constrained-sampling surface (task 20.11): `ToolDef`
  gains a serde-defaulted `constrained_sampling` declaration
  (`json_schema` prefer/require or grammar lark/regex variants),
  `AssistantMessage` gains a serde-defaulted `diagnostics` vector
  (`AssistantDiagnostic` records the strict downgrade and grammar-target
  losses), `ProviderError` gains the non-retryable `StrictToolLimits`
  variant, `AttemptContext` gains `payload_variant` (+ `PayloadVariant`),
  `AnthropicMessagesCompat` gains `supports_strict_tools` and
  `ResponsesConfig` gains `supports_grammar_tools`. Existing struct-literal
  call sites add `Default`/`None` fields; historical records deserialize
  unchanged.
### Added

- workspace: native platform/MSRV and supply-chain delivery acceptance
  (task 20.58, DEL-14): a new `platform_delivery` suite freezes the final
  root Cargo graph (7 members, lockstep internal edges, single `opi-ai`
  feature union, exact build-script owners, the 22-file workspace `unsafe`
  inventory, transitive-only proc-macros, 15 pinned crates with lockfile
  checksums, LICENSE/NOTICE) and asserts declared-host identity; it executed
  green natively on Linux (rustc exactly the declared MSRV 1.97.1) and on
  Windows x86_64-pc-windows-msvc (full offline `opi-ai` suite, 2630 tests,
  with the single parallel-load oauth timing flake classified
  environment-only after standalone and full-suite reruns). The two native
  hosts package `opi-ai` to byte-identical 245-member `.crate` file sets.
  CI is now source-pinned (commit-SHA actions with resolved tags, `--locked`
  gates, an MSRV 1.97 check job, crates.io-pinned `cross 0.2.5`, MSRV-pinned
  release builds) with no dispatch executed, macOS stays an explicit CI
  claim pending an owning host, and a bilingual delivery-evidence baseline
  records every executed outcome and the advertised release-target map.

- `opi-ai`: final assembled external SDK acceptance (task 20.57, DEL-14):
  a new `external_sdk` behavioral suite drives all ten published chat wires
  (Anthropic, OpenAI responses, OpenRouter, Azure, Google, Vertex, Mistral,
  Codex, pi-messages and Bedrock) through one consumer-shaped `Models`
  instance with the full 42-profile builtin registration against local
  scripted transport, plus the four classifier APIs and the sole image API,
  cross-wire `SimpleOptions` portability with typed native-options preflight
  mismatch, assembled request-callback closure (once-per-call observation and
  typed protected-header rejection before the wire), duration measurement
  surviving serialization, a manual tool loop continuing after a serde
  transcript roundtrip, two-instance route isolation over a shared
  credential store with awaited cleanup, empty-instance zero-I/O
  construction and >64-message lawful collections. A new
  `run_readme_examples.py` packaging proof shows the three onboarding
  examples ship byte-exactly inside the `.crate` (embedded verbatim in both
  manuals) and compile as binaries of a fresh workspace-external consumer
  whose dependency graph contains no other Opi crate.

- `opi-ai`: assembled telemetry conformance closure (task 20.55, DEL-12).
  `tests/telemetry_operations.rs` freezes a caller parent Span plus source
  Dispatch under a recording subscriber while a different dispatch is
  ambient at poll time, then proves through the public `Models` seam that
  the frozen pair reaches the producers of every operation family: the
  four chat entries (native/simple/direct plus the advanced `prepare_call`
  path with its acquisition retry and strict-variant rebuild on the real
  anthropic wire), image generation (OpenRouter Images) and classifier
  calls (Typesafe System One) through payload/response observers, and
  deferred submit/fetch/cancel through a custom backend with scoped
  poll/drop sections. Default/omitted and `noop` carriers add no automatic
  SDK spans, adopt no ambient root and leave business results, send, poll
  and callback counts identical to context-free twins; the process-local
  carrier is absent from serialized handles/messages and wire
  bodies/headers, and prompt/secret canaries never appear in any recorded
  telemetry field. Domain status/exporter/flush/evidence stay host-owned;
  no telemetry crate or automatic SDK spans are added.
- `opi-ai`: deferred-response contracts (task 20.50, DEL-11). Submission
  stays a normal generation returning a pending `StopReason::Deferred`
  message with a source-owned `DeferredHandle` (no credential, pi-shaped
  camelCase serialization). `Models::{stream_deferred, fetch_deferred,
  cancel_deferred}` operate on the handle with typed pre-auth validation
  (route identity, caller-clock expiry, unsupported provider), one status
  check at wait0 and never implicit polling; local fetch cancellation is
  independent of best-effort remote cancel. Backends register through the
  operation-qualified `register_deferred_api` seam
  (`DeferredApi`/`DeferredApiHandle`, upsert by exact `ApiId`);
  `test_support` gains `ScriptedDeferredBackend` and
  `MockProvider::with_deferred` for deterministic scripting, and
  `tests/deferred_contract.rs` proves the reopenable consumer checkpoint
  (no resubmission, fake monotonic clock, fixture-only poll advice) and
  AIR-054 telemetry-carrier participation. No real remote deferred
  protocol is claimed.
### Added

- `opi-ai`: builtin provider factory layer with 42 canonical profiles
  (task 20.34, DEL-10). `providers::builtin_provider_profiles` carries the
  source-qualified 42-entry table (canonical ids `google`/`google-vertex`/
  `amazon-bedrock`/…; no legacy aliases) and
  `provider_collection::register_builtin` registers the bundled catalog
  over the completed adapters: mapped providers per profile (azure and
  google-vertex config-gated with typed skip reasons, bedrock lookup-only
  behind its AWS-chain descriptor), a `canonical_route` wrapper republishing
  fixed-id adapters under canonical ids with the model-spec prefix rewrite,
  catalog entries converted with tiered pricing into `ModelInfo::pricing`,
  and the classifier/image protocols registered from catalog records
  through the typed seams; env keys resolve at dispatch through an injected
  `AuthContext` (no environment/store/network reads at construction).
- `opi-coding-agent`: explicit 32 MiB Windows main-thread stack reserve for
  the `opi` binary. The MSVC default 1 MiB reserve left the non-interactive
  async turn pipeline (already mitigated once via `Box::pin` at the
  `run_non_interactive` call site) without headroom; after the task 20.34
  construction layers, unoptimized builds overflowed the main-thread stack
  (`0xC00000FD`) mid-turn. The reserve is a linker argument emitted by the
  new `build.rs` for MSVC targets only.
- `opi-coding-agent`: canonical provider ids with reentrant legacy reference
  migration (task 20.34, DEL-10). Provider prefixes are canonical
  (`google`, `google-vertex`, `amazon-bedrock`); legacy
  `[providers.gemini|vertex|bedrock]` blocks and `defaults.model` specs
  migrate in memory at load (adoption when the canonical block is
  unconfigured, recorded conflict with the canonical block winning when
  both are explicit, reentrant, no file rewrites). Credential-store account
  references follow the same shape: metadata-only canonical-first probing
  with legacy fallback, conflicts detected before any copy, and an
  explicit never-overwriting `switch_provider_account`. The historical
  adapter ids stay internal and are never registered as aliases.
- `opi-ai`: OpenAI Decisions classifier adapter and four-protocol closure
  (task 20.49, DEL-11). `api::decisions::OpenAiDecisionsApi` (api id
  `openai-decisions`, provider `openai`) serves OpenAI's Decisions API:
  one `POST <base>/decisions` with `{model, input, questions}` where the
  state rides as a JSON string (with images: one user message of
  `input_text` plus `input_image` data URLs, max 128 rejected typed
  before any request), `choice`/`score` questions keep their options and
  levels (empty descriptions off the wire, alphabetical criteria order)
  and `bool` becomes a `predicate` whose criteria meanings join the
  instructions. Answers arrive as an array matched by name; refusals,
  missing answers, mistyped shapes and non-finite required numbers are
  typed errors retaining the billed usage and its catalog-tiered cost
  (source-required validations only). API-key-only availability (empty
  key fails typed), gateway 504 never retries and explains the ~600K
  token limit instead of surfacing Cloudflare's HTML page, while other
  failures keep the shared retry policy; the curated `gpt-6-luna` record
  documents the long-context pricing tier above 272K tokens; `temperature`
  stays off the typed options (the wire never reads it). With this
  adapter the classifier seam proves one shared conformance across all
  four real protocols (typesafe, Cloudflare, llama.cpp, Decisions) in a
  single `Models` dispatch.
- `opi-ai`: llama.cpp classification by label-token readout (task 20.48,
  DEL-11). `api::llama::LlamaCppClassifyApi` (api id `llama-cpp-classify`,
  provider `llama.cpp`) classifies with an ordinary chat model served by
  llama.cpp's `llama-server`: each question renders under single-token
  answer labels and the answer is the softmax over the labels'
  pre-sampling next-token log-probabilities (typed temperature divides
  the logprobs post-wire; the completion always carries `temperature:
  0`). `/tokenize` resolves label tokens (newline-prefixed first, alone
  as fallback; split labels and shared tokens are typed errors, cached
  per server root and model), `/apply-template` renders with thinking
  disabled (an open `<think>` block closes at once), `/completion` reads
  the top `n_probs` escalating 256→4096→32768 when a label is missing
  (never ranked is an error, not an invented zero). Answers keep the
  shared semantics (bool probability, score expected value with TypeSafe
  confidence, choice argmax with verbatim distribution); the `/v1`
  OpenAI-compatible suffix strips from the base URL; option counts,
  non-positive temperature, image input and unsupported callback
  families reject typed before the first request; the shared
  retry/timeout policy applies per endpoint; no usage (llama-server
  reports none).
- `opi-ai`: Cloudflare Workers AI System One classifier adapter (task
  20.47, DEL-11). The second real classifier protocol drives the same
  shared conformance through a transport core parameterized out of
  `api::system_one` (URL, request envelope and response unwrap differ;
  guards, payload freeze, retry policy, per-attempt timeouts,
  billed-usage retention and tiered pricing are identical). One POST
  `<base>/run` with the `{model, input}` envelope, the
  `{CLOUDFLARE_ACCOUNT_ID}` endpoint placeholder materialized from the
  resolved account identity (kept literal when absent), Cloudflare's dual
  response envelopes (third-party run records gated on
  `state: "Completed"`, Cloudflare-hosted direct output, `success: false`
  error aggregation), curated `typesafe/jev`/`@cf/cloudflare/clef`/
  `@cf/cloudflare/clef-flash` baseline records, and multi-model
  registration: `Models::register_classifier_model_record` appends to (or
  replaces in) the registration's record set with a typed
  `ModelsError::ClassifierApiNotRegistered` rejection for unregistered
  APIs, and `Models::classifier_model` scans the set.
- `opi-ai`: typed classifier operation and TypeSafe System One direct
  adapter (task 20.46, DEL-11). New public `classifier` contracts
  (`ClassifierContext` with ordered typed questions, `ClassifierAnswer`
  variants, `ClassifierResult` where provider failures ride `stop_reason =
  Error`/`Aborted` with the typed cause and a billed request keeps its
  usage, operation-qualified `ClassifierModelRecord` with tiered catalog
  pricing), the object-safe fixed-`O` `api::ClassifierApi` trait, and
  `Models::register_classifier_api`/`classifier_model` (chat models and
  unknown ids reject typed before any auth resolution)/`classify`/
  `classify_native` with `ClassifierApiHandle::options`. The real
  `api::system_one::TypesafeSystemOneApi` (api id `typesafe-system-one`):
  POST `{model, state, questions}` to `<base>/systemone` with the public
  `bool`↔wire `noul` mapping on both sides, per-question parsing that
  validates only source-required shape/type/string/finite constraints,
  image input rejected before any request, no default timeout with a fresh
  body-inclusive per-attempt timeout, two extra retries by default with the
  SDK retry policy, chat-style tiered usage pricing, and payload/response
  callback support with typed rejection of the raw-event/header-transform
  families.
- `opi-ai`: typed image operation and OpenRouter image direct adapter
  (task 20.45, DEL-11). New public `images` contracts (`ImagesContext`/
  `ImagesOutput`, `ImagesResult` where provider failures ride
  `stop_reason = Error`/`Aborted` with the typed cause, base-rate
  `ImageCostRates`/`ImagesCost`, operation-qualified `ImageModelRecord`),
  the object-safe fixed-`O` `api::ImageApi` trait, `Models::
  register_image_api`/`image_model` (chat models reject typed before any
  auth resolution)/`generate_images`/`generate_images_native` with
  `ImageApiHandle::options`, and `api::openrouter_images::
  OpenRouterImagesApi` (api id `openrouter-images`): nonstream Chat-wire
  POST with record-derived `modalities`, first-choice ordered text-then-
  data-URL parsing that skips non-matching entries, text-only/empty/
  zero-choice successes, usage cache split, base-rate-only pricing, the
  600000 ms body-inclusive per-attempt timeout (positive-only override),
  zero-extra-retry default with the SDK retry policy (server-delay cap,
  frozen payloads across retries), cancellation to `Aborted`, and
  payload/response callback support with typed rejection of the
  raw-event/header-transform families.
- `opi-ai`: pi-messages complete chat adapter (task 20.44, DEL-11). New
  public `pi_messages::PiMessagesChatApi` registers through
  `Models::register_chat_api` (api id `pi-messages`, provider id
  `radius`) with typed `PiMessagesOptions` (reasoning with the `"off"`
  wire value, tool choice auto/none/required/named, `debug`). The wire is
  pi's own message protocol — one POST of `{model, context, options}` to
  `<base>/messages` with an exact six-field options whitelist where
  omissions stay omissions — and the context is the canonical chronology
  projected into pi's camelCase wire JSON (signatures, redacted thinking,
  parsed tool arguments, mid-conversation system messages preserved). The
  simple entry forwards the requested thinking level unclamped
  (`resolve_wire_cache_retention` maps cache retention one-to-one with the
  legacy `PI_CACHE_RETENTION=long` ambient opt-in). Streaming validates
  typed and fail-closed (sparse indices, unknown events, missing terminal
  all fail; `[DONE]` ignored; CRLF and trailing frames parse), maps the
  full usage family with `responseId`/`providerThinkingLevel` and the
  `pi_messages_rewrite` diagnostic, and fails terminal `error` events
  typed (`aborted` to `Cancelled`). HTTP errors format
  `"{status} {statusText}: {error.message ?? body} ({code})"`; callbacks
  fire payload → response → raw parsed events.
- `opi-ai`: mistral-conversations complete chat adapter (task 20.43,
  DEL-11). New public `mistral::MistralConversationsChatApi` registers
  through `Models::register_chat_api` (api id `mistral-conversations`,
  actual POST `{base}/v1/chat/completions`) with typed `MistralOptions`
  (full tool-choice enum including `any`/`required`/named,
  `reasoning_effort` none..max, `prompt_mode`). The neutral entry maps tool
  choice and resolves the reasoning branch from the model record (explicit
  level maps send `reasoning_effort` after clamping with the off entry and
  the `high` fallback; mapless reasoning models send
  `prompt_mode: "reasoning"`). Prompt caching rides the session
  (`prompt_cache_key` + `x-affinity`, explicit header wins, `Disabled`
  suppresses). The response deadline covers only the header wait (default
  60s). Tool-call ids normalize to nine alphanumeric characters on replay;
  streaming keeps the first non-empty chunk id, skips empty content deltas,
  accumulates tool calls by index with derived ids, splits cached prompt
  tokens, maps the Mistral stop names (`model_length` included), fails
  unmapped and in-stream `error` stops typed, and fails a stream without a
  finish reason. HTTP errors format `Mistral API error (status): body`
  (4000-char truncation); 429 keeps its retryable identity.
  `ThinkingLevelMap::has_explicit_mappings` joins the public surface for
  the mapless-versus-mapped branch. The OpenAI-compatible Mistral profile
  is unchanged.
- `opi-ai`: bedrock-converse-stream complete chat adapter (task 20.42,
  DEL-11). New public `bedrock::BedrockConverseStreamChatApi` registers
  through `Models::register_chat_api` (api id `bedrock-converse-stream`)
  with typed `BedrockOptions`: tool choice including the named-tool mode
  (`none` drops the whole toolConfig), the level-based reasoning entry with
  custom thinking budgets (adaptive vs budget-based Claude shapes, GovCloud
  display/binding omissions, gpt-oss/gpt effort fields), interleaved
  thinking and display controls, and the verbatim `requestMetadata`
  passthrough. The neutral entry shares the numeric budget policy with the
  anthropic wire (default table plus answer-room clamp). Cache points ride
  supported Claude models (system + last user message, one-hour TTL on
  `Long` retention); `<empty>` placeholders keep required content arrays
  non-blank; thinking replays as `reasoningContent` with signature/redacted
  semantics; consecutive tool results merge into one user message and
  Claude error-tool images follow the turn's results. `BedrockProvider` is
  `Clone` and gains `with_models`/`with_native_options`; its producer runs
  under the explicit frozen `TelemetryContext`.
- `opi-ai`: google-vertex complete chat adapter (task 20.41, DEL-11). New
  public `vertex::GoogleVertexChatApi` registers through
  `Models::register_chat_api` (api id `google-vertex`) with typed
  `GoogleVertexOptions`: the Google tool-choice/thinking shapes plus
  Vertex-own `project`/`location` that re-route the ADC URL (inert under the
  Express API-key route). A Bearer dispatch without project or location now
  fails typed before any network work. The shared Google body engine is
  wire-aware for the neutral budget policy: the vertex table has no dedicated
  flash-lite entry, so `2.5-flash-lite` rides the flash table (minimal 128,
  not the generative-ai 512). `VertexProvider` is `Clone` and gains
  `with_models`/`with_native_options`; its producer runs under the explicit
  frozen `TelemetryContext`.
- `opi-ai`: google-generative-ai complete chat adapter (task 20.40,
  DEL-11). New public `gemini::GoogleGenerativeAiChatApi` registers through
  `Models::register_chat_api` with typed `GoogleGenerativeAiOptions`
  (native `tool_choice` auto/none/any and the `thinking` shape where
  `budget_tokens` keeps the source semantics — `-1` dynamic, `0` disabled —
  and a level wins over a budget). Thought signatures capture and replay on
  text, thinking and tool-call blocks (base64, same-provider/same-model,
  signature-only empty parts kept); Gemini 3 unsigned/duplicate tool call
  ids regenerate locally; Gemini 3+ strict tools select VALIDATED function
  calling over the original `parametersJsonSchema`; tool-result images nest
  inside `functionResponse.parts` on Gemini 3+ and ride a separate labeled
  turn earlier; consecutive function responses merge into one user turn;
  the stream captures the first non-empty `responseId`, maps thought parts
  to thinking blocks and unmapped finish reasons to typed failures with the
  raw reason; the producer runs under the explicit frozen
  `TelemetryContext`.
- `opi-ai`: anthropic-messages complete chat adapter (task 20.39, DEL-11).
  New public `anthropic::AnthropicMessagesChatApi` registers through
  `Models::register_chat_api` with typed `AnthropicMessagesOptions` (native
  `thinking_enabled` tri-state, `thinking_budget_tokens` with the source
  `|| 1024` default, `thinking_display` defaulting summarized,
  `effort` with typed preflight on adaptive thinking,
  `interleaved_thinking` default true, and `tool_choice`
  auto/any/none/named). The provider consumes the options on its dispatch
  clone; the producer now runs under the explicit frozen `TelemetryContext`.
  Inline-tools chronology (`inline-tools-2026-09-15`): with the new compat
  flags the request tool list stays the initial active set plus the deferred
  placeholder and later system messages carry by-value `tool_addition`
  definitions (same-name redefinitions included) and `tool_removal`
  references, held back before assistant boundaries so `tool_result` pairing
  survives. Error-turn image relocation aligns to the pinned
  `splitErrorToolResultImages` shape (see Breaking Changes).
- `opi-ai`: azure-openai-responses dual-wire adapter (task 20.38, DEL-11).
  New public `azure_openai::AzureOpenAIResponsesChatApi` registers the
  typed direct/native adapter through `Models::register_chat_api` with
  `AzureOpenAIOptions` (native `reasoning_effort`/`reasoning_summary`/
  `tool_choice` plus `azure_deployment_name`/`azure_resource_name`/
  `azure_base_url`/`azure_api_version`; `service_tier` deliberately absent —
  source omission). Public resolution functions
  `resolve_azure_base_url`/`resolve_azure_api_version`/
  `resolve_deployment_name` implement the pinned source precedence
  (explicit → resource → model base URL, typed failure when unset, Azure
  host path normalization, api-version default `v1`, deployment map
  `modelId=deployment`). The provider dispatches by record wire: responses
  records reuse the exact 20.37 Responses parameter assembly under Azure
  transport (deployment URL, `api-key` header, same-identity signed
  reasoning replay, terminal duration), chat records keep the existing
  chat transport, and both producers run under the explicit frozen
  `TelemetryContext`.
- `opi-ai`: complete openai-responses chat adapter (task 20.37, DEL-11).
  New public `openai_responses::OpenAiResponsesChatApi` registers the
  typed direct/native adapter through `Models::register_chat_api` with
  `OpenAiResponsesOptions`: native `reasoning_effort` levels (typed
  rejection before auth; the simple entry clamps),
  `ResponsesReasoningSummary` (`Auto`/`Detailed`/`Concise`/`Null`;
  omission and explicit null both wire the default `auto`, a summary
  without effort defaults the effort to `medium`), `ResponsesServiceTier`
  (`auto`/`default`/`flex`/`priority`/`fast`) and full `ResponsesToolChoice`
  (`Required`/named stay native). Running an effort requests
  `include: ["reasoning.encrypted_content"]` (`xai` reasoning models
  always do); reasoning items persist their serialized wire item as the
  thinking signature and replay pushes it back into `input`; terminal
  output backfills signatures the per-item frames omitted. Catalog compat
  `supportsMidConvoEffort` replays recorded levels as `configuration_update`
  items (stamps before earlier same-model replies plus one at the end,
  request effort fixed at `medium`, replies record
  `provider_thinking_level`). Long cache retention sends
  `prompt_cache_retention: "24h"` beside the clamped `prompt_cache_key`;
  terminal `Done` messages carry the measured `duration_ms`; the producer
  runs under the explicit frozen `TelemetryContext`.
- `opi-ai`: complete openai-completions chat adapter (task 20.36, DEL-11).
  New public `openai_chat::OpenAiCompletionsChatApi` registers the typed
  direct/native adapter through `Models::register_chat_api` with
  `OpenAiCompletionsOptions` (full OpenAI `tool_choice` shape including
  `Required`/named forcing, native `reasoning_effort` levels and per-level
  `thinking_budgets`); wrong-wire records and unsupported native effort levels
  fail typed preflight before auth. The eleven declared thinking formats
  project their on/off/null/omission wire shapes; tool-result images project
  to a trailing user message; signed assistant thinking replays through its
  declared reasoning field (`reasoning`/`reasoning_content`/`reasoning_text`)
  or serialized `reasoning_details`; empty assistant turns are skipped; and
  successful streams carry `duration_ms` from response activation to terminal
  (setup failures stay untimed). The bilingual crate README records the
  adopted native/simple option inventory (ONB-03).
- `opi-ai`: instance resource generation, pool cleanup and awaited local
  release (task 20.17, DEL-07 slice). New public `codex_resources` module
  (P20-RESOURCES seam): a `SessionResourceOwner` cleanup surface exposed by
  providers through the new default-None `Provider::session_resources`
  hook, `Models::cleanup_session_resources(Option<&str>)` aggregation, and
  typed `CleanupReport`/`CleanupError`. Every Codex WS-family call registers
  one resource generation before any auth attach or header/body transform;
  the cleanup cutoff cancels matching old prepared/connecting/busy/
  temporary/active-fallback work, awaits its local owned exits (guard drops
  observed through a revision watch - never a dropped-flag claim), forbids
  late publish, bounded retries and the late SSE leg, and closes removed
  pooled connections source-style (Close frame, flush, short bounded
  confirmation; failures reported as unconfirmed effects while the
  remaining release continues). `Some(nonempty id)` cancels only that
  session (unmatched is a no-op that retains idle reuse), `None` cancels
  everything including sessionless generations, and `Some("")` is a typed
  zero-effect failure. Dropping a call starts its release (producer work is
  cancelled with it) but never claims barrier completion; caller parent
  tokens, external owners, credentials and catalogs stay untouched, and
  pure `Sse`-preference calls remain unregistered.
- `opi-ai`: real Codex WebSocket/Auto/continuation public path (task
  20.16, DEL-07; RDF-24). New neutral `Request::transport`
  (`TransportPreference::Auto` default, plus `Sse`, `Websocket`,
  `WebsocketCached`) - omitted and explicit `Auto` follow the same rules,
  correcting the source's unnormalized local default. The Codex adapter now
  drives the real chain Models -> Codex adapter -> source-qualified strict
  Upgrade -> private frame/PMD/Close owner: `response.create` over a masked
  text frame, events decoded through the exact SSE classifier (identical
  public events), explicit client Close on temporary connections, EOF never
  counted as success, `Request::timeout` as the WebSocket idle timeout, and
  active/idle cancellation. Session-scoped pooling with continuation
  (sessionless/Disabled stays temporary): a delta request is sent only when
  the non-input fields are unchanged and the previous full input plus the
  actual response items form an exact prefix, otherwise the full input;
  continuation publishes only after a successful terminal with a response id.
  Bounded branches: exactly one full-input retry on
  `previous_response_not_found` before public output, exactly one reconnect
  on `websocket_connection_limit_reached` before generation starts, and a
  WS->SSE fallback only for transport-class failures before the first
  effective provider event (per-session sticky); abort/API/auth/protocol
  failures never masquerade as transport loss and an output-delivered error
  ends that stream preserving the partial. `WsUpgradeRequest::headers`
  extends the public Upgrade bridge with caller handshake headers
  (handshake-managed names reserved). The transitional unused-code boundary
  on the private ws modules is replaced by this genuine production path.
  Packaging proof: the ws family now ships a real-chain consumer check
  (fresh workspace-external `Models::stream_simple` -> Codex -> Upgrade ->
  private codec against a local scripted peer) running in all three feature
  graphs and reported separately; `cargo package` artifacts contain the
  private protocol sources, retained tests/fixtures, LICENSE/NOTICE, with
  registry checksums plus AWS-LC and C-libz native provenance recorded.
- `opi-ai`: process-local telemetry carrier `TelemetryContext` (task
  20.54, DEL-12; P20-TELEMETRY owner). `from_span(tracing::Span)` freezes
  the parent span together with its own `Dispatch` through
  `Span::with_subscriber`; observation never adopts creation-time or
  poll-time ambient dispatch. A disabled span degrades to the explicit
  `noop()` state. `Clone` retains the same pair and the context is
  `Send + Sync`. `in_scope(FnOnce)` runs the synchronous body exactly once
  under the frozen pair and restores the caller context; `instrument(Future)`
  returns a lazy `Unpin` wrapper (boxed inner, zero `unsafe`) whose every
  poll and whose drop - including an unpolled drop - run one short scoped
  section installing the frozen dispatch and entering the parent for the
  section only: no enter guard crosses an await, each outer poll polls the
  wrapped future exactly once, `Pending` and the output are preserved. The
  disabled carrier runs the same sections under a no-op dispatcher:
  business work still executes exactly once while section observation is
  explicitly silenced. New direct dependency `tracing-core = "0.1"`
  (workspace): the locked tree already carried tracing-core 0.1.36 as
  tracing 0.1.44's dependency, so this only grants direct visibility.
  No global subscriber/exporter/sampling, domain span vocabulary or automatic
  SDK root span is installed; nothing is serialized or sent on the wire.
  Optional `Request::telemetry_context` propagates through the real Codex
  producer and callbacks; omission captures no implicit ambient context.
  Noop observation is scoped and does not disable independent host logging.
  Full-operation propagation remains owned by task 20.55.
- `opi-ai`: scripted multi-operation test support on the existing
  MockProvider (task 20.53.8, DEL-11; no second mock driver, public API
  unchanged). New `MockResponse::DelayedAfter { pending_attempts,
  retry_after_ms, events }` (pinned faux `pendingFetches`) withholds the
  events while the first `pending_attempts` prepared attempts each fail
  with `ProviderError::RateLimited` carrying `retry_after_ms` - the
  chat-level analog of the pinned `pollAfterMs` hint, reusing the
  existing retry mechanism instead of timers; `pending_attempts: 0`
  behaves exactly like `Events`. New `MockResponse::HangingUntilCancelled`
  yields no events until the request's cancellation token fires and then
  terminates with `ProviderError::Cancelled`, failing immediately when
  pre-cancelled (the attempt-boundary form of the pinned
  signal-aborted-to-error behavior), built with
  `futures_util::stream::once` over `CancellationToken::cancelled`. New
  `blocks_response(Vec<AssistantContent>)` fixture emits one event per
  block in order (`TextDelta` for text, `ToolCallEnd` for tool calls,
  `content_index` matching position) and a `Done` with `StopReason::
  ToolUse` when any tool call is present, else `Stop` - the ordered
  multi-block/tool shape the single-purpose helpers cannot express; it
  panics for `Thinking`/`RedactedThinking` blocks by contract. Existing
  `Events`/`Error`/`EventsThenError` semantics and the
  `stream_prepared` queue-assertion are unchanged; deferred records and
  their scripts stay with 20.45-50.
- `opi-ai`: public header merge, scoped provider env, and cancellable
  waits (task 20.53.6, P20-S53-6/7). `provider_headers::
  merge_provider_header_sources` (pinned `providerHeadersToRecord`)
  merges override sources case-insensitively - later sources win and
  re-insert at the end preserving the winner's original case, `None`
  deletes without inserting, an empty result is `None` - as unvalidated
  config normalization feeding `ProviderHeaders::try_new`, without
  touching `merge_request` fail-closed override rejection. New
  `provider_env::provider_env_value` (pinned `getProviderEnvValue`)
  resolves a name from a scoped injected map first, then the ambient
  process environment, faithfully short-circuiting empty values to
  fallback/`None` like the pinned `||` chain; lazy per-name reads with
  zero construction-time access and no global caching or mutation, no
  proxy/identity policy (owners stay `http`/`auth`), and no Bun
  `/proc/self/environ` fallback. New `cancel::{operation_token,
  sleep_cancel, race_with_cancel, Cancelled}` (pinned `sleep`,
  `raceWithAbortSignal`, `operationSignal`) reuse `CancellationToken`:
  one-way parent-to-child token linkage, timer/listener release on
  cancel/ready/drop, immediate pre-cancelled failure without polling the
  operation, and dropping the abandoned operation future as the
  Rust-native abandoned-promise handling. Deterministic tests use the
  paused Tokio clock and drop sentinels; no new dependency, no executor
  or global timer authority.
- `opi-ai`: public compatibility `short_hash` and UUIDv7 generation
  (task 20.53.4, P20-S53-4/5). `identity::short_hash` (pinned `shortHash`)
  reproduces the pinned JavaScript algorithm exactly - UTF-16 code units
  (surrogate pairs included), 32-bit wrapping multiplies, the sequential
  self/cross final mix, and the unsigned base36 rendering of `h2` then
  `h1` - locked by fixtures; it is documented as non-cryptographic and
  not a durable identity promise, keeping source-qualified crypto hashes
  (PKCE, AWS signing) with their owning modules. `identity::UuidV7Generator`
  (pinned `uuidv7`) owns the monotonic state: `generate` takes an injected
  now-in-milliseconds clock and never regresses on rollback, `generate_at`
  preserves an explicit integer timestamp in `0..=2^48-1` without advancing
  the ordinary rollback floor, the 41-bit sequence is seeded from random
  bytes and incremented until exhaustion, which fails with the typed
  `UuidV7Error::SequenceExhausted` (deterministically reachable via the
  `test_support::force_sequence_exhausted` hook). No new dependency and no
  generic identity framework.
- `opi-ai`: public context-overflow, recoverable-Length and typed
  diagnostic predicates (task 20.53.3, P20-S53-3). `is_context_overflow`
  (pinned `isContextOverflow`) matches the pinned provider overflow error
  patterns after applying the pinned non-overflow exclusions, gates the
  bodyless Cerebras 400/413 shape on the `cerebras` provider, and detects
  silent Stop overflow and zero-output Length-stop overflow against a
  caller-supplied context window - both usage-based branches require known
  usage (`Usage::reported`); unknown usage is never converted to zero
  tokens. `is_recoverable_length` (pinned `isRecoverableLength`) reports a
  Length stop below the original desired output limit under the same
  known-usage rule. Both are pure predicates and never invoke compaction or
  retry policy. `extract_diagnostic_error` (pinned
  `extractDiagnosticError`) returns a serializable `DiagnosticErrorInfo`
  with the `ProviderError` variant name, its secret-safe display message
  (detail carriers are `ProviderErrorSummary`), and the typed service-fault
  HTTP status and wire error name; no raw provider payloads and no stack
  field (no Rust execution-stack analog).
- `opi-ai`: public JSON repair and portable schema construction utilities
  (task 20.53.2, P20-S53-2) in `opi_ai::message`: `repair_json` (pinned
  `repairJson` semantics: raw control characters inside strings become
  valid escapes, invalid escape backslashes are doubled so model text stays
  literal, legal escapes pass through), `parse_streaming_json` (pinned
  `parseStreamingJson` cascade in exact order - empty input, full parse
  with repair, partial-prefix completion of the raw text, of the repaired
  text, then a valid empty object; display-only, never a final authority),
  `finalize_tool_arguments` (public delegation to the AIR-048 finalize:
  final object/`arguments_error` stays authoritative and the raw text is
  retained), and `string_enum` (portable `{"type": "string", "enum": [...]}`
  schema construction with optional description/default, no anyOf/const or
  TypeBox symbols). The partial-prefix completer is implemented in-repo;
  no new dependency.
- `opi-ai`: public visible-text extraction helpers for the message content
  enums (task 20.53.1, P20-S53-1): `content_text` (user `InputContent`),
  `assistant_content_text` (`AssistantContent`) and `output_content_text`
  (`OutputContent`) extract and join text blocks in exact block order,
  separated by single newlines. Images, tool calls and thinking in both
  plain and redacted forms are never exposed. The helpers are read-only
  projections and complete the pinned `utils/text.ts` family together with
  the existing transcript-owned system renderers (`system_message_text`,
  `render_system_message_update`), which render the canonical
  body/section chronology and request-time update framing in exact order
  without changing canonical input.
- `opi-ai`: explicit whole-call retry helper (task 20.52, AIR-065/V-57):
  the new `opi_ai::retry_assistant_call` runs a caller-owned produce
  closure (each invocation a NEW logical call — never a re-dispatch,
  transcript mutation, tool re-execution or prepared-budget reset) with
  bounded retry on transient failures. Omission or a disabled policy
  means exactly one call; `max_retries` counts extra complete calls.
  Classification follows the pinned source pattern lists with
  quota/billing/usage-limit exhaustion vetoing first, then the transient
  families (provider load/status 429/5xx/520/524, gateway forwarding such
  as "unable to forward upstream" and "service temporarily unavailable",
  network/transport, premature stream endings, explicit mid-stream retry
  guidance, gRPC `ResourceExhausted`, subscription usage hiccups). Valid
  stops (Stop/Length/ToolUse/Deferred) and aborts never retry; a failed
  retry budget returns the final error message. Callbacks observe the
  exact order — `on_retry_scheduled` → cancellable exponential backoff
  (caller-supplied schedule, delay cap defaulting to 60s) →
  `on_retry_attempt_start` → the new call — with `on_retry_finished`
  exactly once at loop end, only when a retry was scheduled; a failing
  callback stops the loop with the typed
  `WholeCallRetryError::Callback`. Cancellation during the backoff
  returns the failed message re-stamped `Aborted` (error text dropped,
  partial content and usage preserved). The helper itself never switches
  provider/model/auth and never wires product recovery: host policy
  stays with the host.
- `opi-ai`: strict／grammar wire projection and the single bounded
  downgrade variant (task 20.51 follow-on, AIR-040/AIR-042/V-34/V-36, DEL-04
  ordinary slice). The private wire schema conversion (`make_strict_json_schema`,
  pinned to the source semantics) forces optional→required+nullable
  recursively on a clone — the trusted schema is never modified — with
  per-wire keyword admission; `prefer` projects non-strict when unsupported
  while `require` fails typed before any dispatch, including a fail-closed
  guard on the wires without strict projection (Chat incl. Mistral/
  OpenRouter, Bedrock, Gemini/Vertex, Azure). The one source-qualified
  Anthropic strict-limit 400 (invalid_request_error with the documented
  grammar-size / strict-tool-count / union markers on an actually dispatched
  prefer-strict tool) is typed `ProviderError::StrictToolLimits` and admits
  exactly one rebuilt variant through the acquisition state machine:
  prefer-strict removed, require kept, per-variant budget ≤1+N, total
  ≤2×(1+N), N=0 still two sends, no reset path. The rebuilt variant re-runs
  `onPayload` and post-validation under its own freeze slot, keeps the
  frozen headers (the header transform now runs once per logical call) and
  auth/model, never re-runs the header transform, and records one
  `anthropic_strict_tools_fallback` diagnostic on the message. Other 400s,
  callback errors, cancellation and post-Start failures never branch.
  Responses/Codex wire grammar tools as `custom` tools (lark preferred over
  regex; unsupported targets fall back to plain JSON with the loss recorded)
  and stream `custom_tool_call` input wrapped incrementally into
  `{"<property>":"…"}` with monotonic/single-close enforcement and the raw
  input as the authoritative final arguments; replay restores
  `custom_tool_call(_output)` items with `ctc_` ids while function calls
  keep `fc_`. Returned explicit nulls normalize back onto the original
  trusted schema. Response callbacks observe each adapter's source-defined
  response point; Codex SSE also observes non-success statuses before
  classification.
- `opi-ai`: the credential-store mutation surface moved from IO-free
  primitives to store-owned serialized mutations (P20-STORE / task 20.22).
  `CredentialStore` gains `modify` and `mutation_barrier` (default
  implementations; stores with shared or persistent state must override);
  `CredentialStoreError` is now `Clone`; `Models::logout` submits one
  serialized deletion mutation and acknowledges its receipt (typed
  `ModelsError::MutationOutcomeUnknown` for lost windows) instead of
  calling `delete` directly; `CredentialResolver::resolve_oauth` takes
  `Arc<dyn OAuthProvider>` (the rotation runs detached inside the store)
  and its refresh network bound is now 15s, scoped to the HTTP exchange
  only — lock waits and the ACK are outside it.

- `opi-ai`: functional request callbacks (task 20.51, AIR-059/V-51
  ordinary slice). `callbacks::RequestCallbacks` carries four observer
  families wired through `Models` and every migrated real direct adapter
  (Anthropic, OpenAI Chat incl. Mistral/OpenRouter profiles, OpenAI
  Responses incl. Azure, Codex, Bedrock; Google wires support the payload,
  header-transform and parsed raw families and reject response observers with a
  typed `UnsupportedCapability` before dispatch). `onPayload` observes
  the post-projection wire body — `Ok(None)` keeps it, `Ok(Some)` may
  replace it under validation (JSON object, unchanged model identity,
  output budget never raised) — and runs once per payload variant, with
  ordinary acquisition retries reusing the frozen body byte-for-byte.
  `onResponse` fires with status and headers at each adapter's source-defined
  response observation point; Codex SSE also notifies non-success responses
  before classification. Failures without a response are not notified. `onRaw`
  observes each parsed provider wire event strictly before its
  normalized emission, and a raw failure ends the call with the partial
  preserved. The header transform applies once per logical call to the
  assembled per-call set; protected identity/auth/signing values already
  present may be returned unchanged, while introduction, removal and changes
  fail typed. Adapter-owned credentials stay outside the transform. Shared
  definitions retain independent freeze state across calls, and async builders
  await each callback in source order.
- `opi-ai`: sampling and pricing-tier metadata with the shared resolution
  mechanism (task 20.35, AIR-062/V-45). `ModelInfo` gains
  `sampling_params` and `sampling_params_by_thinking_level` with
  `resolve_sampling_params` per-key precedence (model base < effective
  thinking level < request) applied by the OpenAI-compatible Chat and
  Responses adapters as the last body step, overriding named fields;
  `Request` gains `sampling_params`. `ModelPricing::effective_for_usage`
  selects the highest strictly exceeded tier over
  input+cache-read+cache-write (keeping the first of equal duplicates),
  `calculate_cost_tiered` applies the whole-request tier rates plus the
  response service-tier multiplier (flex 0.5×, priority/fast 2×,
  `gpt-5.5` 2.5×), and `ServiceTier` parses from the Responses wire;
  OpenRouter image `parse_usage` keeps base rates as the explicit source
  exception. Metadata alone adds no pre-dispatch limit.
- `opi-ai`: dynamic catalog store and refresh substrate (task 20.33,
  V-27/AIR-002/AIR-033). New `catalog` items: `ModelsStore` — the single
  revision-commit owner persisting distribution-schema-v1 snapshots (plus
  the `x-opi` fidelity extension) with restore-before-network,
  superseded/cancelled late-publication guards, and last-known retention
  across reopens; `DynamicCatalogSource` — the public seam for host-plugged
  catalog sources; `HttpCatalogSource` — a real controlled HTTP source
  fetching one distribution-schema-v1 document with pooled connections and
  redacted failures. `Models` gains `set_models_store`,
  `register_catalog_source`, `refresh_with(token)`, and per-entry
  `RefreshReport`/`RefreshOutcome`/`RefreshStatus` reporting. The SDK
  registers no source, attaches no store, and has no pi.dev/default startup
  fetch policy.
- `opi-ai`: offline bundled model catalog substrate (DEL-10, task 20.32).
  The new `catalog` module embeds the authentic Pi published model catalog
  as offline resources, packaged byte-for-byte by the explicit
  delivery-input tool `scripts/pi-catalog/pack.py` from the exact revision
  pinned by the local Pi 1.3.0 tree (`sha256-ffa3e986…678`; 42 providers,
  1608 chat / 59 image / 38 classifier entries). The bundle manifest
  records the full provenance chain — pi.dev source URL and revision,
  artifact SHA-256, `@earendil-works/pi-ai@1.3.0` package/local-origin
  association, data schema version 6, bundle schema version 1, provider
  and operation identities, and the digests of Pi's own
  `model-data.ts`/`hydrate-model-catalog.ts` plus the packer.
  `Catalog::builtin()` validates the embedded bytes once per process with
  no network I/O, account probing, or generator runs, re-verifying digest,
  schema versions, provider set, every entry's data contract, `type:id`
  uniqueness, and the structure digest — any mismatch is a typed
  fail-closed `CatalogError`. Operation-qualified getters expose
  chat/image/classifier entries (typed core fields plus the complete raw
  entry); `CatalogHandle::try_install` validates a candidate completely
  before an atomic swap, so tampered, stale, wrong-version,
  missing-provider, duplicate-identity, and contract-violating candidates
  are rejected while the last-known revision stays readable and held
  `Arc<Catalog>` values are never rebound. Explicit complete custom records
  remain usable independently of catalog state. Missing authentic material
  remains a blocking input: nothing invents catalog data from synthetic
  fixtures or implicit construction. Substrate for the ModelsStore
  restore/refresh/personalization and profile/catalog integration owners;
  closes no store acceptance scenario.
- `opi-ai`: `OAuthProvider::{name, is_subscription, login_label}` expose
  read-only metadata on trait objects. All nine SDK flows retain their fixed
  Pi values and optional-field differences; custom implementations default to
  id/None/None. `RadiusOAuthProvider::with_identity(id, name)` captures the
  flow's own identity while retaining its gateway; existing constructors keep
  `radius` / `Radius`, and external Models registration keys remain independent.
  Selection/provenance/typed errors use that identity; refresh preserves the
  selected account binding and never discovers a sibling gateway. Both fresh
  and refreshed Radius request auth ignore credential endpoint overrides. No generic
  inventory or profile enumeration is added.
- `opi-ai`: `AdcNativeOwner` is directly registerable as an `AuthResolver`.
  Its configuration check uses the selected context and file presence without
  parsing credentials or refreshing tokens. `AuthResolver::check_configuration`
  is optional for opaque custom sources; Models propagates check failures and
  reports unconfigured sources as absent. The product Vertex check preserves
  its Express/API-key and static Bearer priority before consulting ADC.
- `opi-ai`: `Models::register_oauth_route` binds a concrete provider, its SDK
  OAuth flow and attached store without source I/O; public
  `auth::store_oauth::StoreOAuthResolver` also supports explicit route assembly.
  `OAuthProvider::to_auth` converts an acknowledged credential without I/O;
  Copilot derives the request endpoint from its token while preserving Bearer.
  `CodexOAuthProvider::with_agent_name` sets browser `originator` (default
  `opi`). Offline assembly fixtures cover the real eight-family adapter wires
  after public login/store ACK/get_auth, plus Radius gateway-bound auth and its
  explicit unavailable-generation boundary. Radius pi-messages generation,
  complete profile catalogs, final packages and platform acceptance remain
  later work.
- `opi-ai`: the Radius gateway OAuth flow landed as the ninth SDK OAuth
  family (task 20.31), pinned to pi 1.3.0's `radius.ts`, plus the
  DEL-08/DEL-09 and V-43/V-55 assembled source/flow conformance. The new
  `auth::oauth::radius` module fixes the gateway at construction (pi
  `normalizeRadiusGatewayUrl`: default https scheme, trailing slashes
  stripped) and mirrors pi's method prompt through the existing
  `select_login_method` seam with browser recommended. Browser mode
  discovers `GET {gateway}/v1/oauth` for the authorization endpoint
  (browser mode only; device mode never discovers first) and runs the
  shared PKCE skeleton with the corpus's fixed loopback callback
  `http://127.0.0.1:1456/oauth/callback` under a required bind (a busy
  port fails the login directly; no paste-back-only fallback), the fixed
  `pi-gateway` client, `gateway offline_access` scope, and the
  `handoff=url` authorize parameter. Device mode posts
  `/v1/oauth/device` and reuses the shared RFC 8628 machine: the
  verification URI passes through verbatim (no scheme validation, unlike
  xAI), denial keeps the single RFC spelling, and non-ok token answers
  classify by error code (pending/slow_down server-interval-or-+5s,
  expired/denied revoked, unknown typed without body echo). Token
  responses require the full access/refresh/expires_in triple on every
  grant; refresh is the shared strict single pass (no RFC 6749 §6
  refresh-token preservation — that tolerance stays xAI's row 146
  fixture) with 401/403 revoked for relogin. The stored expiry is the
  real service lifetime (pi's 60-second write-time skew is not
  reproduced; the single five-minute near-expiry correction stays in
  `needs_refresh`). The assembled conformance test registers all nine
  delivered families on one `Models`, shares one store across two
  `Models` instances, and proves per-id ACK isolation plus late-write
  rotation isolation across sources without re-implementing any family.
- `opi-ai`: the xAI (Grok/X subscription) OAuth flow landed in the SDK
  (task 20.30), pinned to pi 1.3.0's `xai.ts`. The new `auth::oauth::xai`
  module runs RFC 8628 against `auth.x.ai` (form-encoded
  `/oauth2/device/code` with the fixed client id, the xAI scope, and
  `referrer=pi`; `/oauth2/token` for polling and refresh) and keeps pi's
  exact trust and parsing edges: verification URIs are https-only, an
  untrusted `verification_uri_complete` fails rather than coalescing onto
  the plain URI (pi validates before choosing `complete ?? plain`), and
  the device authorization requires a positive finite `expires_in` while
  `interval` falls back to the shared 5s default. Polling branches on the
  ok status first (a success body must carry non-empty access and refresh
  tokens; the RFC 8628 error-code switch, including xAI's
  `authorization_denied` spelling beside `access_denied`, runs only on
  non-ok answers) and follows the shared machine: first-poll wait, 1s
  floor, slow_down honoring a server interval or adding 5s, device
  deadline typed timeout. Token `expires_in` defaults to 3600s. Refresh is
  a single pass (no retry loop on this wire): a response omitting
  `refresh_token` preserves the stored one (the sdk-contracts row 146
  fixture; empty or non-string still fails typed), `expires_in` defaults
  to 3600s, and 401/403 is a revoked session for relogin. The stored
  expiry is the real service lifetime; the single five-minute near-expiry
  correction stays in `needs_refresh`. Failure bodies are never echoed;
  statuses and fixed OAuth error classes are. The presenter receives the
  user code with the complete-or-plain https URI; pi's interval/expiry
  notification fields stay host UX per the 20.28/20.29 seam decision.
- `opi-ai`: the Meta (Muse subscription) OAuth flow landed in the SDK
  (task 20.29), pinned to pi 1.3.0's `meta.ts` plus the shared
  `device-code.ts`. The new `auth::oauth::meta` module runs RFC 8628
  against `auth.meta.com` (trailing-slash `/oidc/device/authorization/`
  and `/oidc/device/token/`, form bodies, fixed client id) and splits
  identity from API access exactly as pi does: the polled identity token
  never authenticates inference; it is exchanged for a Model API key via
  `POST api.meta.ai/muse-code/key` (Bearer identity, empty JSON body,
  `x-api-version: 1.0.0`), stored with the identity token as `refresh` and
  a fixed 24-hour client-side expiry, so the shared scheduler re-mints
  through `refresh` with no refresh_token grant (that endpoint does not
  exist on this wire - the token path is never hit during refresh) and no
  retry loop. A 401/403 from mint - login or refresh - is a dead session:
  typed revoked for relogin, never an account fallback. The device start
  prefers the trusted `verification_uri_complete` and falls back to the
  trusted `verification_uri` (pi coalesces at the trust level: an
  untrusted complete URI defers to the plain one); interval/expires fall
  back to 5s and the 15-minute device deadline; polling follows pi's
  shared machine (first-poll wait, 1s floor, slow_down honoring a server
  interval or adding 5s, typed timeout with the slow-down reason). The
  poll switch reads the body error code regardless of status exactly as pi
  does after its token check. Between identity and mint the presenter
  hears pi's progress notification. Failure bodies are never echoed;
  statuses and fixed classes are. The one corpus-sanctioned exception:
  a keyless mint response may surface a trusted http(s) setup
  `action_url` in the presenter reason (sdk-contracts row 145), never the
  raw body. Host command wording (pi's ``/login meta`` hint) stays host
  UX; the typed reason is "sign in again".
- `opi-ai`: the Kimi Code (subscription) OAuth device flow landed in the
  SDK (task 20.28), pinned to pi 1.3.0's `kimi-coding.ts` + shared
  `device-code.ts`. The new `auth::oauth::kimi_coding` module runs RFC 8628
  against `https://auth.kimi.com`: one form-encoded POST to
  `/api/oauth/device_authorization` with the fixed client id, presenting the
  public user code with the COMPLETE verification URI (both verification
  URIs must be http(s)-trusted; `interval`/`expires_in` fall back to 5s and
  the 15-minute device deadline), then form-encoded token polls under pi's
  shared polling state machine - wait one interval before the first poll, a
  1-second interval floor, `slow_down` honoring a server-provided interval
  or adding 5 seconds, and a typed timeout at the device deadline whose
  presenter reason distinguishes the after-slow-down case (pi's WSL
  clock-drift wording is host UX). `authorization_pending` keeps polling;
  `expired_token`/`access_denied` are terminal typed revoked grants; a
  success body must be the strict triple (non-empty access and refresh,
  positive finite `expires_in`) recorded as the real service expiry. Every
  request carries the 30-second deadline; failure bodies are never echoed
  (statuses and fixed OAuth error classes only). `refresh` posts the
  refresh grant with up to three retries over 429/5xx and transport
  failures (1s/2s/4s backoff); 401/403 or an `invalid_grant` error code is
  a dead credential - typed revoked, zero retries, never an account
  fallback. pi's `KIMI_CODE_OAUTH_HOST`/`KIMI_OAUTH_HOST` override is host
  policy expressed through construction (trailing slashes trimmed).
  Host-test maintenance in the same commit: the cancelled-login
  dropped-listener probe now runs only on the anthropic iteration's
  exclusive ephemeral port - the codex flow's shared 1455 preferred port
  with its manual-only bind fallback cannot prove ownership, and the probe
  could strike a concurrent login's listener.
- `opi-ai`: the OpenRouter OAuth flow landed in the SDK (task 20.27),
  pinned to pi 1.3.0's `openrouter.ts`. OpenRouter exchanges an
  authorization code for a permanent, user-controlled API key instead of
  an access/refresh pair: the new `auth::oauth::openrouter` module
  authorizes at `https://openrouter.ai/auth` with `callback_url` + S256
  PKCE and no `state`/`client_id` (the CSRF defense is the fresh random
  `/oauth/callback/<uuid>` route each login, now validated by the shared
  loopback handler, which answers any other route or method with a 404
  page and keeps waiting - a faithfulness strengthening for every PKCE
  flow), races the ephemeral-loopback callback against pasting the full
  redirect URL, a `code=` fragment, or the bare code (each strictly
  parsed; empty input is a typed failure), and exchanges via one JSON
  POST to `https://openrouter.ai/api/v1/auth/keys` (`code`,
  `code_verifier`, fixed `code_challenge_method`, 30-second deadline)
  whose response must be JSON carrying a non-empty `key`. The credential
  stores the key as the access token with an empty refresh slot and the
  permanent sentinel expiry (pi's `MAX_SAFE_INTEGER` mapped to the last
  representable instant, 9999-12-31, because `time` tops out at year
  9999; `expires_at: None` could not be used since `needs_refresh`
  treats a missing expiry as refresh-now), so a permanent key is never
  rotated: `refresh` is the identity with zero network I/O, and the flow
  never issues the personalization request (`/models/user` stays an
  explicit, separately selected catalog helper). The shared PKCE runner
  gained `AuthorizeStyle` (`callback_url` form vs the classic query) and
  an optional post-bind `notify_progress` template (OpenRouter reports
  "Listening for OpenRouter OAuth callback on <redirect uri>"); the
  existing Anthropic/Codex/ChatGPT constructions are unchanged in
  behavior.
- `opi-ai`: the GitHub Copilot OAuth flow is now SDK-owned (task
  20.26), pinned to pi 1.3.0's `github-copilot.ts`. The new
  `auth::oauth::copilot` module runs the GitHub device-code flow (fixed
  client `Iv1.b507a08c87ecfe98`, `read:user`) against `github.com` or a
  user-entered GitHub Enterprise domain: the new
  `LoginPresenter::prompt_text` (blank = the public deployment; a
  non-empty unparseable answer fails typed before any HTTP) is
  normalized to a bare hostname and all three endpoints are derived from
  it like pi `getUrls`. The device response is strictly validated and
  its `verification_uri` must be an http(s) URL; polling waits one
  interval before the first poll and honors the server interval on
  `slow_down` (else RFC 8628 +5s), with `access_denied`/`expired_token`
  as typed terminal revocations and unknown codes closed. The exchanged
  short-lived Copilot token is strict (`token` + absolute `expires_at`;
  401/403 revoked); the request base URL is derived per request from the
  token's `proxy-ep=` hint (`proxy.` -> `api.`), falling back to
  `https://copilot-api.<domain>` and the individual default - the
  product's `endpoints.api` protocol is deleted. The account's models
  metadata is discovered with bounded 429 retries (`Retry-After` seconds
  or HTTP-date; login 2/5s, refresh a single attempt) and parsed like pi
  (tool-call-less models filtered, picker/policy semantics, the
  enabled-policy fallback only on the individual endpoint, the host's
  static catalog gating which unconfigured models login may enable via
  `{"state":"enabled"}` chat-policy POSTs after a `notify_progress`).
  Policy activation is login-only: refresh re-exchanges the stored
  GitHub token, re-discovers the metadata and never touches policy.
  Credentials gained non-secret `enterprise_domain` and
  `available_model_ids` fields on `OAuthCredential`/`Credential::OAuthToken`
  (persisted atomically by the host keychain envelope), store the real
  service expiry with the single five-minute
  `needs_refresh` correction, and keep the GitHub token in the refresh
  slot. The product `CopilotOAuthProvider` protocol duplicate is
  deleted; the host registry registers the SDK provider with the host
  catalog as the policy whitelist, the TUI presenter answers
  `prompt_text` through the existing line-reader subprocess, and the
  per-request resolver derives the Copilot base URL from the token (a
  legacy stored `base_url` still wins so pre-cutover credentials never
  change endpoints).
- `opi-ai`: the OpenAI ChatGPT direct OAuth flow landed in the SDK
  (task 20.25), pinned to pi 1.3.0's `openai-chatgpt.ts`. The new
  `auth::oauth::chatgpt` module registers a dynamic client per login
  (`client_id=dynamic_agent_client`) and requires OpenAI's issued client
  id on every callback and pasted URL: the form exchange and every later
  refresh reuse it, the credential persists it (`client_id` on
  `OAuthCredential`/`Credential::OAuthToken`), and a stored credential
  without it fails typed with a reconnect error. The authorize URL
  carries the production scopes, S256 PKCE, an independent state and
  nonce, the RFC 8707 `resource=https://api.openai.com/v1` parameter,
  `agent_name_hint` (default `opi`), and the installation's stable
  `ext_agent_host_id=urn:uuid:<lowercase>`. The redirect is the fixed
  `http://127.0.0.1:1455/auth/callback`: a failed bind is a typed
  terminal error (`CallbackBindFallback::Required`; never a free port or
  paste-back ladder), while the bound server races a strictly validated
  manual input that must be the full redirect URL with the same origin
  and path, matching state, and the issued client id. The token response
  is validated per pi `credentialFromTokenResponse` (direct-grant scope,
  positive finite `expires_in`, `id_token` present on login only); the
  stored expiry is the real service expiry with the single five-minute
  `needs_refresh` correction. `run_pkce_login` was refactored into the
  shared `run_pkce_login_with` seam (Anthropic/Codex behavior identical).
- `opi-ai`: the OpenAI Codex legacy OAuth flow migrated into the SDK
  (task 20.24), pinned to pi 1.3.0's `openai-codex.ts`. The new
  `auth::oauth::codex` module owns the browser PKCE flow
  (localhost:1455 with paste-back fallback) and the device-code flow
  (structured pending/slow_down codes, 403/404 pending quirk, 15-minute
  device budget, JWT `codex_account_id` extraction on login and refresh,
  strict refresh fields); the host switched to the SDK provider and its
  endpoint-derived registration, deleting the product Codex flow.
  `mod.rs` gained `CallbackBindFallback::{FreePort,ManualOnly}` and
  `RefreshFieldPolicy::{Lenient,Strict}` on `refresh_oauth_token`.
- `opi-ai`: the Anthropic OAuth flow migrated into the SDK (task 20.23),
  pinned to pi 1.3.0's `anthropic.ts`: PKCE S256 with copy-code and
  browser-callback methods, the 53692 preferred loopback port with a
  free-port then manual fallback, JSON token exchange, required login token
  response fields, the flow's configured client id on refresh, and
  `Models::login` persisting through the store-owned ACK queue. The
  host re-exports the SDK provider.
- `opi-ai`: store-owned credential mutations with ACKs and a synchronous
  mutation barrier (P20-STORE / task 20.22). New `mutation` module:
  `CredentialStore::modify` submits a serialized read-modify-write whose
  receipt (`MutationReceipt`, non-secret) records the applied
  write/delete with per-provider revisions, typed failures
  (`MutationError::{Store, Operation}`), and unknown loss windows; a
  started mutation survives waiter cancellation while the host runtime
  lives, per-provider FIFO serializes login/logout/replace/refresh, and
  there are no default wait, receipt, or admission caps.
  `CredentialStore::mutation_barrier` captures the store-wide cutoff
  synchronously (shared across every `Models` over the same store) and
  reports pending/settled/failed/unknown mutations plus fenced late
  writes without starting auth or aborting rotations; `Models::
  mutation_barrier()` exposes it. New queue-backed
  `InMemoryCredentialStore` reference implementation. The production
  keychain bridge (`opi-coding-agent`) routes `write`/`delete`/`modify`
  through the queue, holds its cross-process `fs4` lock across each full
  slot (host locking policy preserved), and runs OAuth rotation as a
  detached store-owned mutation: a second waiter re-reads the committed
  token instead of double-refreshing, a post-failure re-read under the
  slot reuses a concurrent writer's fresh token, and a rotation queued
  behind a logout reports `CredentialNeeded` instead of resurrecting the
  replaced identity. `/login` persists through the serialized mutation
  before acknowledging success.
- `opi-ai`: lazy AuthContext and auth management results (AIR-063/AIR-066 /
  task 20.18). `AuthContext` is the injectable object-safe source-access seam
  (`env`/`file_exists`) with `AmbientAuthContext` as the lazy default;
  `Models::set_auth_context` replaces ambient access entirely.
  `Models::{check_auth, get_available, get_auth, logout}` deliver the
  management results: checks never refresh (expired stored OAuth stays
  configured), store and source-access failures are typed
  (`ModelsError::AuthContext`/`CredentialStore`/`AuthResolution`), a
  selected-source failure never falls back, an explicit
  `AuthResolutionOverrides::api_key` wins over every source
  (`AuthProvenanceSource::Explicit`), and `logout` acknowledges exactly one
  local store deletion. `opi-coding-agent`'s `/logout` and `--list-models`
  now route through `provider_factory::{logout_stored_credential,
  available_model_entries}`; compatible profiles register config-valid with
  their real env descriptor and availability is decided by the SDK check.

- `opi-ai`: private `ws::frame`, `ws::pmd` and `ws::io` modules own the
  WebSocket client substrate inside the SDK. One exclusive connection
  state machine operates over an upgraded
  `AsyncRead + AsyncWrite` stream; strict `frame` decoding (reserved
  opcodes/RSV, fragmentation, control-frame shape, shortest length encodings,
  64-bit high bit, actual-received-byte typed fallible storage growth);
  owned `pmd` decoding (pre-tail `SyncPoint` proof, exactly-once
  `00 00 FF FF` tail injection, `Z_BLOCK`/`BFINAL`/`ResetKeep` history,
  context takeover, bounded per-poll work, explicit cancellation) over
  unmodified libz-rs-sys 0.6.8 / zlib-rs 0.6.8 / flate2 1.1.10; the explicit
  client Close state machine with caller-supplied masks, retained in-flight
  events and caller-only deadlines; and the fixed 72,413-case deterministic
  PMD boundary corpus replayed against recorded expectations. Directional
  PMD parameters retain both negotiated context flags and windows. The tests,
  fixtures, license and compression NOTICE move into the SDK package; ordinary,
  `flate2/default` and `flate2/zlib` caller feature unions retain the same
  protocol and native maintenance boundaries. No public codec or test feature
  is added. The codec has no production caller yet: this physical move does
  not close real public Codex WS integration (20.16), final SDK package
  acceptance (20.57), or native platforms/MSRV (20.58).
- `opi-ai`: the `ws` module (RDF-24 / task 20.15), the strict dynamic
  WebSocket Upgrade bridge over the source-qualified `HttpClient`:
  request shape with pinned-entropy nonce/mask helpers, and strict typed
  101 validation (status/version, exact `Sec-WebSocket-Accept` with the RFC
  6455 known vector pinned, singleton/duplicate rules, token-wise
  `Connection`/`Upgrade`, offered-only subprotocols, exact
  permessage-deflate parameter negotiation) before detaching the upgraded
  raw stream and negotiation metadata. Upgrade does not construct the private
  codec; provider/resource policy remains separate from protocol state.
- `opi-ai`: the acquisition-retry owner surface (AIR-007, task 20.4):
  `AcquisitionPolicy` (per-call extra sends, hint bound, deterministic test
  jitter seed), `RetryAdmission` (1-based retry index plus the
  source-qualified delay), `AcquisitionSeal` with `retry_admission()` and
  `acquisition_seal()` on `PreparedProviderCall`, and
  `Models::prepare_call_with_policy`. The `retry_backoff` suite proves the
  episode through the real OpenAI Chat adapter: hint-honoring readiness with
  typed zero-dispatch refusal of early starts, the family backoff range
  without hints, exact `1 + max_retries` send caps with exhausted sealing,
  frozen payload/credential reuse across attempts, post-handoff and
  unknown-dispatch sealing, and cancellation forbidding further attempts.
- `opi-ai`: explicit redirect and send policy on the public transport
  (RDF-22, task 20.12). `HttpClientBuilder::redirects(RedirectPolicy)`
  bounds lawful redirect sends (default `Limited(10)`, owned by `opi-ai`);
  exceeding the bound fails acquisition with the typed redacted cause
  `Network("too many redirects")` and no further sends, `None` returns
  3xx responses to caller status classification, and cross-origin hops
  strip protected headers (`authorization`, `cookie`, `cookie2`,
  `proxy-authorization`, `www-authenticate`) while same-origin hops keep
  them. New `http_acceptance` integration suite drives the real
  Models -> Chat -> HttpClient path over scripted loopback peers with
  bytewise and coalesced delivery: exact lawful-send counts, redirect
  chains, typed acquisition versus post-acceptance failure stages,
  single-terminal EOF without retries, the recorded hyper 1.9.0
  acceptance boundaries (8 KiB initial read buffer with adaptive growth,
  100 head fields, 16 KiB cumulative chunk extensions, upstream trailer
  count/byte constraints and no new SDK body/event cap), HTTPS proxy outer TLS -> CONNECT -> inner
  TLS and scoped NO_PROXY bypass. Offline packaged-consumer proof via
  `scripts/pi-sdk-conformance/package_slice.py --family http --offline`
  (ordinary and caller feature-union graphs, resolved versions and
  registry checksums).
- `opi-ai`: source-scoped TLS on the public transport (RDF-23, task 20.14;
  DEL-06 substrate). `HttpClientBuilder::tls(TlsOptions)` accepts one owned
  typed value for trust selection (packaged pinned Node v22.19.0 default CA
  bundle, extra or explicit roots, explicit host-system-root opt-in), an
  optional mutual-TLS client identity admitted through the private crypto
  owner, and H2-preferred/HTTP1-only ALPN. Inputs are validated and frozen
  before any I/O; the AWS-LC instance provider, WebPKI verification and the
  declared key-exchange order are fixed internally with no process-global
  provider mutation. New `tls_contract` integration suite drives the real
  Models -> Chat -> HttpClient path over loopback TLS (H2/H1 × TLS 1.3/1.2,
  mTLS identity profiles, trust/hostname/EKU/ALPN/no-common-group negatives
  with zero business HTTP, cancellation/partial-body cleanup, host ring
  controls), plus alg-only RS256 JWT sign/verify fixtures and the offline
  `scripts/pi-sdk-conformance/package_slice.py --family crypto --offline`
  packaged-consumer proof (ordinary + caller feature union) with resolved
  graph, registry checksums and native build provenance. Production
  dependency `rustls-native-certs` (OS root store loading, explicit opt-in
  only). The workspace `reqwest` now enables its `http2` feature so the
  shipped client actually supports the profile's H2 path in production
  graphs (previously only reachable through dev-dependency feature
  unification).
- `opi-ai`: canonical replay projection and tool association (DEL-03,
  task 20.7; V-07/V-08/V-15 SDK-active, AIR-010/012/013/014). New `replay`
  module projects the canonical chronology into the request-local wire
  view: association by batch position plus source ID (closed batches may
  reuse IDs; duplicate call/result, orphan result and name mismatch are
  typed failures before any request is built), unambiguous missing results
  closed request-locally as ResultUnavailable/UnknownEffect synthetic
  error results (never NotExecuted, never re-executed, canonical
  untouched), continuation identity by exact wire + provider + requested
  model with cross-identity turns dropping ALL thinking (readable, signed,
  redacted; never converted to visible text - Pi 1.3 semantics) and
  same-identity keeping signed/redacted blocks including signed-empty while
  dropping blank unsigned thinking, failed (Error/Aborted) turns omitted
  with their linked non-execution results as a closed unit while Length
  turns keep expressible pairs, malformed proposals as raw text on
  string-argument wires and explicit named failure contexts on
  object-argument wires (no `{}`/`null` fabrication), request-local
  bijective wire-ID legalization synchronized across call/result, image
  association/order/failure-flag preservation with Anthropic error-result
  image splitting after the turn's results, `details` never serialized,
  URL sources passed through, and `ReplayProjection::Strict` rejecting
  lossy transforms with typed errors. **Breaking (0.x):**
  `AssistantMessage` gains a serde-defaulted `wire: Option<WireApi>`
  provenance field (legacy `None` is never same-identity, fail-closed);
  reported models now ride `response_model` with the requested identity in
  `model` on Chat/Responses/Codex/Gemini/Vertex/Azure/Bedrock (matching
  Anthropic); all six provider `build_request_body`/`build_converse_body`
  entry points return `Result<Value, ProviderError>` and consume
  `project_replay` at the head of request construction (Chat keeps visible
  turn text alongside `tool_calls`; Chat/Responses send raw argument
  strings; Responses parser packs `call_id|fc_item_id` and the serializer
  restores both for same identity, dropping the item id for foreign
  history); real tool-result image blocks on image-capable Anthropic/Chat
  routes replace MIME-note placeholders. Host policy (execution,
  authorization, retry, session readers/writer) is unchanged; AIR-038
  per-wire encrypted-reasoning parity and V-32 effort stamps close with
  later tasks.
- `opi-ai`: canonical Context/Transcript chronology (DEL-03, task 20.6).
  New `transcript` module ports the pinned Pi 1.3.0 `transcript.ts`
  semantics: the public `Context` shorthand folds once into a leading
  `Message::System` entry via `normalize_context`, the single constructor
  of `TranscriptContext` (stored transcripts re-enter through the
  transparent message list; pre-cutover lists parse unchanged); later
  system messages append `content`, patch ordered named `sections`
  (replacement keeps position, `None` deletes, delete+readd appends) and
  change tools by value (`tools_added`/`tools_removed`; a redefinition is
  removal+addition; equal states add no redundant change). Replay resolves
  the current prompt/tools; projection is native-or-collapse returning a
  fresh value (canonical history never rewritten), with explicit
  `TranscriptProjection::Strict` rejecting unsupported temporal placement
  before any request is built. Declaration installation validates
  non-empty unique names and dialect-engine schema compilation before a
  change enters the transcript (a keyword inert under the structural guard
  is not an admission error). Documented Rust boundaries: system `content`
  is a plain string; declaration equality is structural (key-order
  independent, `1` vs `1.0` distinguishes where JavaScript would not —
  extra deltas only, never missed ones); upstream-deprecated
  `hasToolRedefinitions` is not ported.
- **Breaking (0.x, AIR-020):** `provider::Request` now carries
  `context: TranscriptContext`; the independent `system` and `tools`
  runtime fields exit in the same scoped break. All six provider adapters
  plus api-mapped/Bedrock consume only the normalized transcript via
  `resolve_transcript` (collapse is the current per-wire capability,
  expressed as a named per-wire constant for later compat tasks); the
  agent loop folds its convenience prompt and registry definitions through
  the seam; `Models::prepare_call` derives budget inputs from the replayed
  current state; compaction summary requests, harness/RPC gated-provider
  summary detection, diagnostics/validators, MockProvider recorded reads
  and the affected tests migrated in the same compiling cutover. Host
  policy (hooks, authorization, compaction thresholds, session v1
  readers/v2 writer) is unchanged.
- `opi-ai`: public tool-argument normalizer/validator seam
  (`schema::validate_tool_arguments`/`validate_tool_call`, DEL-04 task
  20.10). Ports the pinned Pi 1.3.0 `validateToolArguments` pipeline:
  owned `arguments_error` rejects before coercion, clone-before-normalize
  keeps the caller's raw value untouched, optional-null deletion via
  independently compiled sub-schemas (compile failures keep the key,
  `$ref` properties skipped), TypeBox `Value.Convert` is a no-op for plain
  JSON dialect schemas (`~kind` out of the dialect, documented), and the
  upstream coercion walk (allOf fold, anyOf/oneOf value-first with
  clone-per-arm, type-union primitive coercion in `type`-array order,
  object properties + object additionalProperties, tuple/single items).
  Number conversion uses a private ECMAScript StringNumericLiteral
  classifier (full StrWhiteSpace set, signless 0x/0b/0o with exact
  round-to-nearest-even, correctly rounded decimal, `Infinity` rejected by
  the finite gate); string conversion uses the annex-pinned `ryu-js` 1.0.2
  for floats and exact integer display (RDF-09 precision). The normalized
  root is always checked against the original schema — the upstream's
  return of unvalidated raw on a failing coerced root is not replicated
  (RDF-09 fix, asserted against the recorded bug case). Invalid outcomes
  carry redacted failures (instance pointer, keyword, schema-derived
  reason); the evaluator grew observation-only first-failure localization
  (JSON-pointer paths threaded through the work-stack frames; acceptance
  unchanged, proven by the frozen 64-case corpus). The frozen 44-case
  public differential battery asserts the recorded Pi normalized outputs
  (one RDF-09 exact-integer divergence documented in-case).
- `opi-agent`: tool validation now consumes the SDK seam. `RegisteredTool`
  owns its compiled schema IR generation (lazy `OnceLock`, replaced
  registries compile anew, in-flight calls keep theirs); preflight runs
  compile+validate on the blocking pool under biased cancellation (result
  discarded, no synchronous hard-kill claim); hooks observe the raw
  proposal while the authorizer and `Tool::execute` receive the normalized
  value (the exact validated value is the value authorized and executed).
  The `jsonschema` dependency leaves the tool-validation path (unrelated
  `opi-protocol` uses retained).
- `opi-ai`: private TypeBox JSON dialect schema engine (`schema` module,
  DEL-04 substrate). Admission compiles every structurally-selected regex
  leaf (fail-closed, with JSON-pointer paths) and selects fast vs reducer
  evaluation by structural presence of `unevaluatedItems`/
  `unevaluatedProperties`; the evaluator ports the pinned TypeBox 1.3.27
  `Compile` semantics — all 42 JSON mechanisms, the 21 default string
  formats (including the upstream IDNA hostname family, punycode and NFC
  normalization), JavaScript key-enumeration order, full
  lexical/resource/dynamic/recursive reference frames (WHATWG URL base math,
  anchors, retrieved/resolved resource frames, legacy draft-4 bases;
  unresolved references evaluate invalid, not error), per-branch evaluated
  keys/indices with successful-arm merge, `uniqueItems` via the upstream
  FNV-1a-64 value hash, and the upstream grapheme-cluster length rules.
  Evaluation uses an explicit work stack (schema depth never exhausts the
  Rust call stack), types unchanged-instance self-recursion as a typed
  `NoProgress` failure while productive recursion continues, and has no
  implicit I/O or source-free size/fuel limits. The frozen 64-case research
  differential corpus retains its original inputs; two UCD-17 cases now assert
  the approved upstream-data outcomes, with the historical oracle preserved.
  `serde_json` switches to `preserve_order` workspace-wide (insertion-order
  maps, the selected dialect identity); mechanical serialization-order
  fallouts were updated deliberately (eval manifest digest key order,
  execution-v1 schema snapshot, one clippy large-Err allowance). New
  dependencies: regress 0.12.0 (defaults off; std, utf16, prohibit-unsafe),
  unicode-normalization 0.1.26.
- The private Unicode16 vendor source family and its generator, integrity and
  local-registry checks are retired. Schema uses the ordinary crates.io URL
  graph already shared by the SDK; no private package identities are required.
- `scripts/pi-sdk-conformance`: offline package and external-consumer checks
  exercise the public SDK with ordinary registry dependencies, including caller
  feature unification. Frozen Node observations remain historical evidence;
  current expectations explicitly identify accepted upstream-data differences.
  Complete conformance and final distribution acceptance remain open.
- `opi-ai`: byte-exact stream framing across all current real adapters
  (AIR-006). One shared incremental replacement decoder replaces per-chunk
  lossy decoding: multi-byte UTF-8 split across network chunks survives
  byte-exactly, each maximal invalid subsequence becomes one U+FFFD instead
  of a fatal SSE UTF-8 rejection (Anthropic previously failed the call),
  Bedrock payloads replace instead of silently emptying, valid keepalive
  comments and large lawful events beyond the withdrawn size caps stay
  accepted, and AWS prelude/CRC validation with checked arithmetic is
  preserved (proven at every byte split).
- `opi-ai`: final tool-argument parsing on the five textual wire sites
  (Anthropic, Bedrock, Mistral, Chat and shared Responses, with inherited
  adapters). `ToolCall` gains an optional serialized `arguments_error`:
  empty input finalizes to `{}` without an error, input differing only by
  invalid escape sequences is repaired so escaped text stays literal and
  later fields survive, and incomplete or non-object finals keep the owned
  raw text plus the error, never a partial-prefix success. Streaming partial
  JSON stays provisional.
- `opi-ai`: cancellation now takes priority over buffered events on every
  adapter (V-04). Anthropic, OpenAI Chat, OpenAI Responses and Codex
  Responses migrate to the cancel-aware queue/consumer seams already used by
  Azure, Gemini, Vertex and Bedrock: cancelling at a confirmed wait point
  (read or full queue) delivers exactly one typed cancellation without
  pre-cancel buffered events penetrating, and dropping the stream ends that
  attempt's delivery without affecting a fresh attempt.
- `opi-ai`: one `Models` front door for chat calls (AIR-045). Native
  `stream`/`complete`, neutral `stream_simple`/`complete_simple`, direct
  `stream_direct`/`complete_direct` (injected, already-resolved auth) and
  advanced `prepare_call` share validation, auth resolution, the attempt seam
  and `ChatStream` collector. Native options retain adapter budget semantics;
  simple and advanced preparation apply the shared budget. Ordinary calls capture the owned
  input, the caller's complete record, and the registered adapter at
  construction (`Send + 'static`), and authentication starts on the first
  poll; pre-auth failures (unregistered ApiId, options type/provider
  mismatch, non-dispatchable route, cancellation) surface as exactly one
  typed `CallFailure` and never reach auth.
- `opi-ai`: typed custom chat registration. `ChatApi<O>` adapters register
  with an exact `ApiId` and a fixed options type; the registration installs
  a catalog-invisible route, so the adapter's complete record stays absent
  from `get_model`/`all_models` yet remains callable without catalog
  membership, and same-ID catalog data never replaces a frozen caller
  record. `ChatApiHandle::options` tags values with their API/type identity,
  so the same option value crosses `Models` instances with compatible
  registrations; an incompatible upsert fails old options with a typed
  pre-auth mismatch. Already-created calls keep their captured adapter
  across later registration changes.
- `opi-ai`: `ChatStream` and `CallFailure` are exported at the crate root;
  `ProviderError` is now `Clone` + `PartialEq`.
- `opi-ai`: private upstream crypto substrate (DEL-06, task 20.13;
  RDF-23). New private `crypto` module owns key admission and signing on the
  unmodified crates.io baseline `rustls` 0.23.40 + `aws-lc-rs` 1.18.2
  (`aws-lc-sys` 0.46.0) through the public safe seams only
  (`KeyProvider::load_private_key`, `SigningKey::algorithm`/
  `choose_scheme`, `Signer::sign`); no local FFI, crypto fork/namespace, or
  public crypto API is added. The profile admits unencrypted two-prime
  RSA-2048..8192 keys (PKCS#1/PKCS#8, PEM/DER, including e=3) and P-521
  keys (PKCS#8/SEC1) with independent public-key verification; RSA-1024,
  multi-prime RSA, encrypted/malformed/public-only inputs and non-profile
  algorithms are typed, redacted rejections without false success or key
  leakage. The AWS-LC provider is instantiated explicitly per owner and
  never installed process-globally; transitive `ring` coexists and existing
  HMAC/SHA/SigV4 signing is unchanged. `aws-lc-sys` builds the vendored
  AWS-LC C sources into a static library (standard evidence:
  `AWS_LC_SYS_USE_SYSTEM=0`); untested feature unions/platforms remain
  unsupported. Cancellation-aware signing runs on an owned blocking worker:
  cancellation is observed between operations only, owned keys/resources
  are retained until the worker joins, and late results are suppressed
  rather than published. Substrate for the TLS/mTLS host wiring (20.14) and
  real ADC callers (20.21.1); closes no product acceptance scenario.

## 0.8.3 - 2026-10-15

### Breaking Changes

- `opi-agent`: `AgentLoopConfig.max_turns` is removed. The generic loop has
  no intrinsic turn ceiling; embedders provide explicit budgets through
  hooks or cancellation, and `PrepareNextTurnContext.turn` is now `u64`.
  Stop hooks observe the completed turn before queues or preparation;
  preparation runs only for an already-required continuation and cannot
  itself request one. Initial steering precedes the first request, and
  terminating tool batches suppress automatic continuation without
  suppressing queued input. Complete candidate validation and atomic
  replacement remain enforced.
- `opi-coding-agent`: `DefaultsConfig.max_iterations` is now `Option<u32>`,
  defaulting to no turn-count ceiling. Explicit budgets remain product-owned
  and fail before a required extra turn; zero fails before provider dispatch
  instead of reporting an empty success. Natural completion at the
  configured limit still succeeds. Evidence distinguishes no applicable
  turn quota with the new `UnknownReason::NotApplicable` variant from a
  numeric quota.
- `opi-coding-agent`: `DefaultsConfig` adds the optional `max_output_tokens`
  field, and optional `[defaults] max_output_tokens` sets the output limit
  on each provider request independently of model capability metadata. It
  must be positive, fit the selected model's output capability and exceed
  any enabled thinking budget; zero, over-ceiling values and incompatible
  thinking budgets fail before provider dispatch. The limit survives
  retries, tool continuation and compatible model/thinking changes;
  incompatible runtime changes reject without persisting. `ConfigError`
  adds `InvalidMaxOutputTokens`, and `CodingHarness::set_model` now returns
  `Result<(), String>`, rejecting incompatible model/request settings
  before changing the active model.
- `opi-coding-agent`: local bash execution has no default deadline, matching
  Pi. `DefaultsConfig::tool_timeout_ms` and `BashRequest::timeout` are
  optional; an explicit default or per-call timeout still applies, and
  cancellation retains process-tree cleanup. External adapters require an
  explicit finite timeout before activation because their protocol requires
  a deadline.
- `opi-coding-agent`: automatic compaction now runs before model requests,
  using current context occupancy, a default 16384-token reserve, roughly
  20000 retained tokens and a real model summary. `threshold_tokens` is
  optional and explicit thresholds remain binding. Summary generation and
  atomic message/marker/Leaf commits preserve the current run, cancellation
  and turn budget; summaries inherit thinking and retry settings, omit
  image bytes and bound individual tool-result text. Manual compaction
  methods are asynchronous and RPC compaction remains cancellable. Session
  compaction markers add optional provider/model/usage metadata for summary
  accounting; older records remain readable with unrecorded usage treated
  as unknown.
- `opi-ai`: `Models::prepare_call` now resolves an omitted
  response output limit from the selected model's capacity before
  authentication and rejects default thinking budgets that cannot fit. Both
  implicit and explicit ceilings are clamped to estimated remaining context
  with 4096 safety tokens; thinking is fitted inside a reduced ceiling,
  leaving 1024 answer tokens where possible. The shared `context` estimator
  counts system/tools, images and applicable same-model usage plus new
  messages rather than lifetime usage. Direct adapters retain their
  behavior.
- `opi-coding-agent`: omitted response limits now follow model capacity
  instead of provider fallbacks or thinking-budget-plus-one. Default
  medium/high thinking budgets are 8192/16384 tokens; oversized startup
  thinking budgets leave answer room. Explicit output controls remain
  binding across model changes and tool continuation.
- `opi-ai`: `AssistantContent::Thinking` now carries an optional
  `signature`, and `RedactedThinking { data }` preserves encrypted
  continuation blocks. Rust constructors must supply the signature field;
  old unsigned serialized messages still deserialize. Anthropic streams
  retain both kinds of blocks through sessions and same-provider/model tool
  continuation. Model aliases retain the requested identity separately from
  the reported response model. Missing reported model names remain unknown
  in actual-route evidence. Exports omit opaque metadata; foreign routes
  receive only readable text.
- `opi-agent`: validated preparation supports an acknowledged external
  commit before installing context, with run-scoped summary evidence and a
  preparation-only operation. A durable acknowledgement remains the rollback
  boundary when cancellation or a later request fails. Hook wrappers forward
  the transaction methods and their single-committer declaration.
- `opi-eval`: native mode now binds every score to the selected Agent's
  same-trial work, and the native contracts move to
  `opi-eval-native-material/7`, `opi-eval-native-handoff/4`,
  `opi-eval-run-plan/2` / `opi-eval-plan-preview/2`,
  `opi-eval-native-agent-terminal/3` and `opi-eval-owned-resources/2`.
  One explicitly selected upstream job per trial runs the selected Agent
  inside the graded task environment and grades that same trial; the
  imported result must bind to the selected Agent, task and job identity.
  The former host-agent then independent default/oracle-job score path, the
  unchecked newest-job result admission, the appended `answer.txt` prompt
  glue and `run --preflight-only` are removed. Complete pinned corpora are
  independent of the per-task material map; each selected task binds its
  own native name, profile, package and immutable images, and a profile's
  task ID binds to its exact admitted upstream task name so generated trial
  IDs stay canonical. Prepare caches per-task profiles and packages, and
  both run entry paths dispatch exactly the selected tasks. Optional
  runtime packages bind the complete resource tree and executable
  entrypoint; Harbor and Pier verify and transfer the full package,
  preserve Agent output before cleanup and launch the selected connection's
  provider/model, while local preparation accepts package directories under
  `agents/<product>` and rejects resource drift. A local `runner_project`
  pins and re-verifies the installed `.venv/bin/python` interpreter and
  `.venv/bin/<runner>` console entrypoint bytes; missing or changed
  installed bytes reject material loading before any validate, plan or run
  effect and before accepting frozen plans, and preparation also pins the
  runner project source and uv environment, rechecks source identity on
  load and uses uv directly for oracle witnesses. Benchmark profile
  inventory keys include the revision so the three families share one cache
  without reporting false profile drift. Headless Opi task processes
  isolate the session keyring so the pinned client's environment-credential
  fallback can use the projected connection; Pier applies its existing
  filtered inference route only to the Agent process. Owned-environment
  evidence binds Docker containers to the exact private Compose working
  directories and verifies removal after every dispatched job, including
  upstream exit-zero failures; unconfirmed cleanup blocks successful
  sealing, and temporary `handoff.json`/`config.toml`/`models.json` are
  removed on settlement while `handoff-identity.json` preserves non-secret
  launch identity. Fresh dispatch requires the `--version` runtime
  preflight before configuration or credential projection; failed or
  unknown preflight prevents task dispatch and remains attempted, invalid
  and unscored, and preflight and task exits are distinct. material/6 and
  older manifests, terminal/2 and earlier records, and run-plan/1 plans
  require regeneration for fresh dispatch; existing sealed bundles are
  preserved read-only for offline verification, reports and verifier-only
  regrade.
- `opi-eval`: private thinking never reaches a first capture sink. A pinned
  musl capture helper (the Companion's hidden `__native-capture` mode,
  staged in handoff/4 before credentials) spawns the selected Agent through
  bounded pipes and persists only validated, privacy-projected bytes under
  both capture policies. Thinking text, declared private signatures and
  `redacted_thinking` data are removed wherever assistant messages repeat,
  private deltas are emptied preserving placeholders/indices, and
  non-private tool/native bindings stay truthful; unknown grammar,
  duplicate keys, invalid JSON, incomplete records or streams beyond the
  existing 64 MiB ceiling refuse without a native score, and opaque agent
  stderr retains only static metadata. The `opi-eval-native-capture/1`
  receipt binds the retained digests, the host re-verifies those bytes
  before any terminal completion, client message, seal or bounded partial
  copy (`opi-eval-partial-capture/2`), and untrusted captures never cross
  into sealed exportable evidence.
- `opi-eval`: the native host credential stage is private before the first
  secret byte: the per-trial integration directory is created exclusively
  with mode 0700 and the staged config/handoff files with mode 0600,
  independent of the caller's root permissions and umask; a preexisting
  directory, file or link target refuses fail-closed instead of being
  reused.
- `opi-eval`: every run freezes its complete report identity under
  `opi-eval-run-control/2` before its first effect - the resolved experiment
  contract and admitted integrity record join the identity, selection,
  corpus and planned-trial bindings - so offline recomputation preserves
  unstarted trials even when no trial sealed, and drifted control bytes
  refuse publication. Trials seal a `control/settlement.json` record of the
  runner's final classification; the offline report consumes it for outcome
  classes instead of re-deriving from stderr tokens or process exits.
  Graded trials (including valid native zero and benchmark-defined Agent
  timeouts) form the per-benchmark metric denominator with its own
  definition and no cross-benchmark average, while
  provider/access/identity/infrastructure/invalid-grading failures stay
  attempted-but-invalid. Published reports gain coverage counts (`corpus`,
  `selected`, `planned`, `attempted`, `valid`, `agent_failure`, `invalid`,
  `unexecuted`), the `full|subset` scope label, repetition control pairs
  (`consistent|divergent|incomplete`) and a protocol-group guard (an edge
  pairing two subjects of one product never carries a paired claim;
  single-subject reports need no artificial comparison edge). Repeated
  reconstruction over identical sealed inputs is byte-identical.
  Normalized reports and reporter identity move to
  `opi-eval-normalized-report/2` / `opi-eval-reporter/2`; binary native task
  outcomes keep their benchmark-specific meanings, and native test-case
  `passed / total` is a separate per-trial observation with raw provenance
  and explicit unknown reasons. Native regrading emits
  `opi-eval-native-regrade/2` / `opi-eval-regrade-record/2`: the offline
  `regrade --root --native-material --out` entry verifies every source
  bundle, requires preserved Agent artifacts to still digest to the sealed
  verifier-input identity, re-grades them with the pinned verifier in fresh
  trial-owned environments without spawning an Agent, resolving a
  credential or contacting a provider (network-requiring verifiers without
  an offline equivalent, tampered bundles, altered artifacts, a drifted
  patch base or drifted task material reject fail-closed), and exits 0 when
  every sealed native trial regraded, 1 for typed per-trial rejections and
  2 for a rejected request (including an existing output root, which a
  regrade never overwrites).
- `opi-eval`: experiment documents move to schema `opi-eval-experiment/2`,
  and admission is fail-closed on enforceable controls. A top-level
  `connection` selects one named `[[connections]]` entry (provider, `api`,
  base URL, exact model, exactly one credential reference); remote routes
  need the explicit `[execution].mode = "live"`, requested monetary/usage
  caps reject as unenforceable, and an explicit positive
  `[execution].max_output_bytes` rejects at resolution because captured
  output stays under the fixed runtime limit. Explicit native `reasoning`
  supports `off`, `minimal`, `low`, `medium`, `high`, `xhigh` and `max`
  through both client projections; required settings need exact-client
  proof of the first request and tool continuation before dispatch, and
  required-but-unsupported controls (reasoning levels, sampling values no
  selected client can express) reject before any model effect while
  optional unsupported settings remain explicitly non-comparable. The
  native-material manifest loses the agents' fixed scripted config and
  inline credential fields: request identity is projected from the selected
  connection into each pinned client's exact configuration bytes, and new
  material requires `agents.<product>.output_control_evidence` - two
  independently observed loopback requests - whenever output limits are
  required; each admitted trial seals that proof for offline acceptance.
  File-backed Opi credentials reach the environment variable selected by
  the isolated client configuration; secrets remain private launch material
  and are removed before verifier execution and evidence export.
  `PrepareArgs` adds paired optional `--opi-source` and `--opi-commit`
  (a cold Opi build requires an explicit local repository and exact
  commit; verified warm packages and pi-only preparation need neither).
  Native `run --plan` reuses its frozen material without `--fixtures`,
  exactly one of `--config` and `--plan` is required with typed exit-2
  rejections for missing inputs, and executing into a previously claimed
  run root rejects before replacing control or trial evidence
  (`--recover` keeps its read-only meaning).
- `opi-eval`: the Python operator modules and sole configuration template
  move from the skill into `crates/opi-eval/operator/`; the development
  entrypoint is `python3 crates/opi-eval/operator/evaluate.py`, and the
  portable skill consumes a supplied `EVAL_OPERATOR` entrypoint and explicit
  private profile. New profiles use `opi-eval-operator/2` while `/1`
  profiles remain readable with unchanged model defaults. The manually
  dispatched external-lock materialization now delegates its diagnostic
  preparation and the explicit oracle-witness conformance to the shared
  local CLI and publishes `opi-eval-materialization-receipt/2` /
  `opi-eval-external-lock/resolved/2` only after both succeed, binding the
  built Companion, diagnostic config, prepare and conformance commands,
  reports and material as contained SHA-bound artifacts; a shared refusal
  publishes no receipt, and legacy `/1` receipts are rejected as read-only
  history.
- `opi-eval`: the offline local-acceptance verifier
  (`crates/opi-eval/scripts/verify-local-acceptance.py`) now requires the
  `/3` manifest contract (`opi-eval-local-acceptance/3`) with an explicitly
  selected independent expected-identity file (`--expected-identities`,
  schema `opi-eval-acceptance-expected-identities/1`). Primary evidence
  needs at least four attributable Terminal-Bench 2.1/3.0 Opi/pi trials -
  one pair per frozen task/group - with complete frozen task/repetition
  coverage; DeepSWE and unknown primary families are rejected (existing
  DeepSWE/Pier capability evidence remains, and unused Pier identity is no
  prerequisite). The verifier validates inline execution-provenance and
  freeze records, original command captures (both streams and argv bound),
  reciprocal attempt coverage and byte-bound build receipts/executables
  before Companion invocation; it rejects substituted, oracle, missing or
  fixture-only evidence, refuses an empty generic-audit success, never
  launches an Agent, reads credentials or contacts a provider, and a
  missing, malformed or mismatched source/build/command/freeze fact
  rejects without claiming authentic provenance from mutual agreement
  alone. Legacy `/1` and `/2` manifests and the weak `command_exits` tuple
  table are no longer accepted; the verification report gains the selected
  expected-file digest, approved freeze digest and an explicit
  build-causality limit. pi-only runtime plus packaging independence (no
  Opi crate dependency, no bundled corpora/graders/images) stays pinned at
  the spawned-CLI seam, and a populated dummy credential environment keeps
  the deterministic hermetic contract unchanged.

### Added

- `opi-eval`: `evaluate.py baseline --bench <id> --tasks <list|@file>`
  prepares a task-subset evaluation: the frozen plan expands only the named
  tasks, records the subset selection and its reason, and the native report
  carries `selection_scope: "subset"` with the excluded corpus remainder.
  Precision re-runs (for example provider-failure victims) no longer
  require a full corpus re-execution.
- `opi-eval`: every finished configured execution automatically exports a
  full bilingual benchmark report to the project's `docs/eval/` - locked
  identity block (provider:model, thinking/output-token controls, corpus
  commit, runner/Companion digests), coverage, the native metric, per-task
  outcome and reward tables, and for Opi candidates the paired comparison
  against the selected pi baseline - and, when the new optional
  `storage.exports` root is configured, copies the compact
  machine-evidence set to `<exports>/<evaluation-id>/` for Windows-side
  consumers. The export is best-effort: failures record a typed gap in
  `export-handoff.json` and never change execution results.

### Changed

- `opi-coding-agent`: JSON mode now writes and flushes events during the
  run, including image input, instead of buffering stdout until prompt
  completion. A dedicated writer acknowledges each line to apply bounded
  backpressure; output failures cancel the run with exit 1, and Ctrl-C
  remains observable while waiting for a slow writer. Forced termination
  retains emitted bytes without fabricating terminal events. New streaming
  runner methods leave existing explicit in-memory capture APIs and NDJSON
  schema 2 intact. The CLI allocates its one-shot future on the heap to
  avoid Windows debug-build main-thread stack overflow before provider
  dispatch.
- `opi-coding-agent`: `--json-compact` now strips redundant cumulative
  thinking, opaque blocks and tool arguments from text/thinking/tool-call
  delta updates, including signatures attached to omitted thinking
  snapshots. Full block-end and terminal messages, ordinary JSON and
  durable/model-facing thinking remain unchanged.
- `opi-coding-agent`: JSON session summaries now include observed token
  usage from failed prompts and estimate known costs from those same
  totals. Both text and content inputs count completed responses once,
  including across repeated runs and resumed history. Unknown usage still
  leaves cost unknown.
- `opi-coding-agent`: non-interactive provider stream failures append a
  summary-redacted `DiagnosticPayload` JSON line to stderr after the human
  message. Captures retain safe transport/protocol causes without exposing
  raw upstream errors; NDJSON framing, exit codes and retry behavior are
  unchanged.
- `opi-coding-agent`: startup resolves the default thinking level against
  the selected model's supported levels before dispatch, using the same
  numeric budget policy as runtime selection. Models exposing only
  low/high/max now start at high instead of rejecting medium. Explicit
  unsupported enabled-level selections remain errors.
- `opi-ai`: Anthropic HTTP streaming buffers complete SSE frames before
  decoding, preserving signed reasoning across UTF-8 and CRLF chunk
  boundaries.
- `opi-eval`: the static external lock now pins the pi coding-agent to the
  upstream `v1.0.4` tag (commit `7c10bd4337495ee613f2224843ecdf349b80d1df`)
  with its verified `package-lock.json`/`package.json` blobs and the
  `@earendil-works/pi-ai` 1.0.4 model-data archive, and the lock's tb21
  image set now binds every corpus task image (89 tasks) to its verified
  immutable manifest so a full-scope `terminal-bench-2.1` selection passes
  image preflight. Upstream removed the published `npm-shrinkwrap.json` in
  1.0.1, so the lock drops that blob and the agent builder records
  `shrinkwrap_sha256: null` when the pinned source publishes none. The
  pinned CLI surface is protocol-identical for the adapter, so the adapter,
  profile, operator template and smoke templates only retarget their
  pinned-version references; `pi:latest` still resolves to this pin at
  preparation.
- `opi-eval`: external `evaluations/YYYY/MM/<eval-id>/record.json` groups
  frozen family operations, bilingual/machine reports and native attempt
  evidence. Discovery reads JSON records, explicit read-only legacy imports
  and lifecycle receipts directly, without SQLite or a persistent query
  cache; existing `catalog.sqlite` files are ignored and left untouched.
  Queries, pinning, retirement, inventory-bound archive/GC previews,
  explicit plan application and canonical-path archive restore keep original
  seals and scores unchanged. Finished native evaluations and standalone
  comparisons apply latest-three rolling retention by Agent, benchmark
  combination, model/API/parameters/thinking and task scope; pi releases
  retain independent buckets, while Opi commits/versions and Companion
  builds do not reset counts. Pins, references, active work and unknown
  cleanup/dependencies are protected, with extra retained entries reported.
  Only owned evaluation trees are maintained; standalone output and shared
  caches remain caller-owned. Current-worktree candidates reuse verified
  eligible pi snapshots without rerunning pi, and `--bench all` uses one
  evaluation and one first-dispatch deadline across families.
- `opi-eval` skill: native reports, sealed evidence, catalogue and
  verification output stay external by default; `docs/eval/` receives a
  bilingual conclusion pair only on an explicit caller request. Native
  benchmark and six-trial operator acceptance modes select software and
  version, benchmarks and model, drive the Rust Companion through frozen
  plans, and stage privacy-reviewed evidence with bilingual reports and
  offline verification. `version=pi:latest` resolves at preparation to the
  static-lock pi pin, Opi-only benchmark runs may reuse a registered sealed
  pi baseline under identity-tuple checks with cross-run pairing disclosed,
  and omitted Opi versions select a build of the current local working tree
  with recorded commit, dirty-source and binary identities. GLM-5.3-Flash
  defaults to a 1000000-token context window and 131072-token per-response
  output limit, with exact-client control proof required before native
  planning.
- Assurance tooling: `opi-audit` accepts optional redacted `evidence_root`
  proof, checks source/client/material applicability and actual execution
  after sealing requirements, installs its independent core before
  compiling a bounded historical reference, and requires only `phase`,
  `reviewer` and `model` (remaining inputs are optional overrides).
  Remediation apply checks the exact approved plan and live audit digests
  against its committed source baseline; commits touching only the current
  Phase's fixed plan pair no longer force a HEAD reseal.
- Development tooling: implementation smoke wrappers now live in
  `.agents/skills/opi-implement/scripts/` with their tests; audit tests
  move beside their owning skill helpers, shared assurance tests move to
  `.agents/skills/_shared/scripts/`, and Eval operator tests move to
  `crates/opi-eval/operator/tests/`. Repository-wide documentation, Cargo
  cache and release artifact-audit tools retain their root `scripts/`
  entrypoints.
- `opi-eval`: local material preparation works from an independent
  directory and is fail-closed about what it freezes. `doctor --config C`
  reports Linux-engine reachability, architecture, WSL/Docker-observed
  CPU/memory/GPU/disk and every selected preparation input `prepare`
  verifies - each with its expected cache location and a present/missing/
  unknown state - with reasons on every unknown observation and no model
  started. `prepare --config C --out M` verifies the seeded local cache
  against the pinned static lock, freezes the selected benchmark, subject
  executables and every requested image identity, checks the frozen task's
  declared resource requirements against local observations (an unmet or
  unobservable requirement rejects the whole request, full scope included),
  and emits a local-origin manifest consumable by `validate` and `run`;
  local origin never manufactures GitHub provenance. Preparation acquires
  missing selected task and Dockerfile images by admitted immutable digest
  and verifies them in the selected engine; corpus preparation admits
  pinned inert metadata links directly beneath `tasks/` only when their
  relative targets are regular tracked files in the same commit; pi
  preparation verifies the model-data archive against an explicit URL and
  SHA-256 and includes the locked external runtime dependencies and Linux
  x64 esbuild binary; and the workspace-input pin matches the committed
  Cargo.lock. Material acquisition settles under a whole-invocation
  deadline with signal cancellation, bounded diagnostic output, recorded
  owned cleanup and no publication after unsuccessful settlement; every
  selected observer (engine, version, disk, GPU, context probes across
  `doctor`, `prepare`, `plan` and `run`) runs under the process supervisor
  with a finite per-probe bound, the 16 KiB capture ceiling and verified
  owned-tree cleanup, so a stalled or overflowing observer becomes a
  truthful bounded refusal instead of an unbounded wait. Selected local
  Docker Desktop storage can be observed on WSL through a bounded read-only
  probe of the already running engine namespace when engine, process,
  mount and endpoint identity agree; unavailable, remote, ambiguous or
  failed observations stay unknown, and the Linux Desktop storage gate
  admits exactly the two known exact Docker Desktop engine
  operating-system identities so the containerized backend reaches that
  observer while near-match and unknown identities stay unknown. Plan creation canonicalizes the
  accepted native-material path so a nested-output relative path loads the
  same unchanged material from any working directory. The native-smoke CI
  delegates materialization to this same shared preparation path and routes
  every CI consumer through the published per-family prepared-material map,
  keeping no independent transform or admission rule.
- `opi-eval`: selection and execution semantics are frozen and bounded
  before dispatch. `plan --config C [--native-material M] --out P`
  enumerates the pinned corpus inventory from `task.toml` metadata without
  pulling or starting images, expands `[selection]` (`scope=full|subset`,
  explicit `task_ids`, recorded `reason`, `repetitions`, optional per-task
  `seeds`; mutually exclusive with explicit `[[trials]]`) into exact
  trials, and rejects the whole request on any unmet requirement while the
  preview keeps per-task statuses for the whole inventory. The frozen
  `opi-eval-run-plan/2` file binds the embedded experiment source, material
  identity, corpus entries and exact trials under one digest; `run --plan`
  re-verifies the digest, material bytes, experiment identity and corpus
  expansion before executing exactly the frozen trials. `[execution]`
  bounds are explicit: `concurrency` is an enforced upper bound, the
  declared `max_trials` caps every resolved trial list (explicit trials,
  selection expansion and replacement) before native acquisition, root
  reservation or dispatch, and the seven integer execution bounds reject
  present non-integer values with the existing typed control error.
  Replacement runs verify and expand the original frozen plan before
  deriving fresh identities, give every actually re-executed trial a fresh
  trial ID (only the targeted comparison group carries the attempt
  suffix), and use a fresh output root; `run --recover` prints a read-only
  classification of every frozen planned trial, including unstarted
  entries. One run-level cancellation token bounds every dispatch:
  SIGINT/SIGTERM stops new affected dispatch, cancels only owned process
  trees, removes the job containers bound to each trial's private Compose
  directories plus temporary credential material, and still settles and
  renders the interrupted report with cancelled and never-dispatched
  trials as explicit missing coverage. Native jobs carry accepted Agent
  and verifier deadlines in one pinned runner configuration (bounds narrow
  each other; the Terminal-Bench 3.0 profile's whole-job deadline is
  18,000 seconds and terminal-bench-2.1's is 15,000s, covering the task's
  Agent window plus grading and lifecycle overhead; watchdog failures are
  ungraded `native-job-timeout` at the infrastructure boundary while
  benchmark-valid Agent timeouts retain their native grade). Pinned
  Harbor/Pier jobs select an owned Docker execution environment for both
  Agent and verifier stages; uncertain termination stops further dispatch
  and blocks sealing, teardown preserves shared images and neighboring
  resources, and mounted jobs write Agent output and Opi traces to the
  trial log mount before completion, retaining a bounded private diagnostic
  copy after outer timeout/cancellation without fabricating terminal
  events, grades or seals. Native Opi configuration omits
  `[defaults].max_iterations`, using the client's default loop without a
  fixed turn ceiling while native deadlines and cancellation remain in
  force. Native cleanup uncertainty and rejected execution identity cancel
  further dispatch while the failed attempt and planned missing coverage
  stay visible and valid Agent failures retain their native grading and
  continuation. Native canary detection and credential guarding cancel the
  run before another trial starts: without `--canaries`, an exact echo of the
  selected credential in any staged native export blocks sealing and never
  serializes the secret into a guard list or artifact.
- `opi-eval`: native scoring and result admission are evidence-bound.
  Scoring uses Opi's provider exit diagnostics and pi's validated terminal
  JSON, including errors reported at exit zero; terminal provider failures
  are excluded from graded denominators while recovered retries, ordinary
  task failures and timeouts retain eligibility. Grading rejects missing
  or substituted terminal product, executable digest and model identities
  before admitting a score, and result admission compares the observed
  task reference to the exact absolute dispatched task package, so a
  substituted package cannot be admitted even when recorded names match;
  a reverse consumer cross-check requires every ordinary file in a
  declared collection to be covered by the integration's terminal
  delivered observations. Result binding reads the pinned Harbor/Pier
  `jobs/<job>/<trial>/result.json` layout (a valid zero reward no longer
  fails because of a nonexistent `trials/` directory), checks Harbor's
  recorded verifier mode, resolves manifest destinations relative to the
  trial directory, and selects the configured primary reward by name so
  Pier's auxiliary test metrics stay separate from trial scores. Sealed
  native Agent timing records an explicit unknown reason when Agent-only
  duration was not observed; offline reports retain bound unsealed
  failures in attempted and invalid coverage without scores or verified
  status, and reward provenance names the exact sealed trial result.
  Offline regrade loads only the products bound by verified source trial
  identities (pi-only and Opi-only material allowed), orders task-package
  manifest entries by relative path to match sealed digests, rejects
  linked trial entries including Windows junctions, and starts verifier
  processes with only their validated material environment so ambient
  provider credentials never reach them. Full-selection reports use the
  frozen corpus task count for selected coverage, and offline native
  comparisons retain unsupported sampling as non-comparable while
  requiring sealed, bound output-control evidence for requested output
  limits. Local acceptance resolves each subject's product from its sealed
  experiment instead of fixed subject names and binds provenance to the
  admitted material: the preserved inventory is mandatory and path-unique
  with exactly one material entry per trial family, family identities
  derive from the admitted lock bytes with the operator identity cache as
  corroborating evidence, and coordinated substitutions, duplicates,
  cross-family bindings and missing or tampered inventories reject.
- `opi-eval`: Terminal-Bench 3.0 and DeepSWE v1.1 native scoring run
  through their declared task handoffs. TB3 uses the separate-verifier
  handoff: the task's `task.toml` lifecycle is parsed and bound into every
  native dispatch (multi-step tasks, disabled verifiers, verifier collect
  hooks on the Harbor route and non-Linux tasks reject before any verifier
  spawn), and only declared artifacts cross into the fresh verifier -
  undeclared, missing, uncollected or substituted collections reject.
  DeepSWE uses the committed-patch handoff: the Pier task's collect hook
  extracts the committed patch beyond the task's base revision and the
  pristine no-network verifier grades exactly that patch (an empty nop
  patch still collects and grades zero); the bound job must record the
  declared lifecycle with the same base revision and image. The sealed
  bundle records the verifier-input identity (`native/verifier-input.json`)
  with the digests of exactly the collected bytes.
- `opi-eval`: the scripted provider serves all three admitted protocols
  (Anthropic Messages, OpenAI Completions, OpenAI Responses) with the same
  deterministic tool loop, model-keyed terminal failure scripts
  (`auth-fail`, `rate-limit`, `retry`, `protocol-error`) and authentication
  evidence in its request log. Agent adapters classify typed terminal
  provider causes from each client's own evidence (including a JSON error
  at exit zero for pi) without speculative parsing, and the pi native
  terminal importer classifies all three exact pinned protocol-truncation
  messages as `provider-protocol-error` with failed outcomes at client
  exit zero. The upstream oracle is an explicitly selected conformance
  diagnostic (`conformance --case oracle-witness`; the incorrect
  `nop-witness` grades zero), never a prerequisite silently run by `run`;
  oracle/nop diagnostics select the pinned native runner's registered
  Agent name, and native oracle/nop conformance witnesses verify exact
  owned container cleanup before reporting, including setup failures and
  timeouts.
- `opi-eval`: native trials seal one interpretation per trial: valid native
  grades (including zero scores and benchmark-valid timeouts) keep their
  native metric, terminal provider/access/adapter/identity/infrastructure
  failures preserve attempted coverage without an Agent score, and rejected
  result admission keeps truthful private diagnostics while refusing
  scoring and export.

### Removed

- `opi-eval`: the legacy `docs/eval/evidence-roots.jsonl` index; its 2026-09
  era records stay in Git history and are referenced as prose by the
  historical audit documents. `docs/eval/README.md` now documents the
  automatic report export workflow instead of the registry.
- Repository: removed the top-level `examples/` package skeletons, their
  manifest smoke tests and the accompanying `docs/extension-examples/`
  documentation. Extension behavior tests remain self-contained under
  `crates/opi-coding-agent/tests/`.

### Fixed

- `opi-eval`: the pinned pi 1.0.4 client is admitted end to end. The pi
  stream importer binds real retry shapes honestly instead of rejecting
  them as `import-unsupported-schema` (the opaque `entry_appended` context
  prune between `auto_retry_start` and the retried `agent_start` is
  tolerated, a recovered chain may be followed by a new retry chain whose
  attempt counter restarts, and an exhausted chain settles as
  `agent-provider-failure:connection`), the native capture helper admits
  the pinned 1.0.4 session-stream shapes (built-in-MCP prompt `sections`,
  tool-change lists, `responseModel`/thinking-level/`rawStopReason`/
  diagnostics fields, `textSignature`, redacted thinking, `role:
  "toolResult"` messages and the declared `toolCall` shape; private
  thinking text and signatures still redact and anything undeclared still
  refuses), and the operator, native projections and Harbor/Pier launches
  accept the complete 1.0.4 thinking-level enum including `max` (integer
  sampling inputs normalize to the Companion's float contract, and a
  lowered OpenAI effort rejects exact-client evidence). Native capture
  stderr admission is whole-record instead of substring-based: an agent
  stderr record is retained only when it is exactly one validated plain
  provider summary or the pinned structured Opi stream diagnostic parsed
  through its closed field set, and everything else keeps the whole stream
  opaque with zero retained bytes.
- `opi-eval`: Harbor/Pier deadline, teardown and settlement paths stay
  bounded and honest. The Agent deadline termination stops only the owned
  Agent launch: an integration-private wrapper launches the selected Agent
  in its own job process group, records its PID with kernel start
  identity, authenticates it on deadline, collects the owned descendant
  set and kills every owned group, while the first-sink capture helper
  survives in its own group and finalizes a genuine sanitized receipt so a
  benchmark-defined Agent timeout keeps its native reward. The
  owned-environment teardown settle bound rises from 15s to 120s, the
  terminal-bench-2.1 whole-job deadline rises from 900s to 15,000s (the
  old outer timeout killed full-window agents before the task-declared
  verifier could grade them), and agent-phase settlement is shielded and
  always publishes the terminal observation (the stop-path bound rises to
  60s, and cancellation yields an honest terminal with an unobserved exit
  that settles one evidence-invalid trial instead of stopping the run).
  Runtime-preflight and capture-helper probes retry once when the
  docker-exec transport returns an empty marker frame, and owned-environment
  cleanup uncertainty reflects the observed final state: the
  post-teardown residue check is the cleanup authority, a transient
  teardown failure is tolerated and its earlier marker removed when it
  confirms no resources remain, while observed residue, a pre-existing
  marker from another phase or a failed settle still marks cleanup
  uncertain. The integrations record an honest `unreadable` capture
  summary when the first-sink receipt is absent or unparseable instead of
  `null`, keeping the trial a settled evidence-boundary invalid while the
  run continues.
- `opi-eval`: honest no-score native shapes bind instead of aborting runs.
  The agent/verifier-timeout `Unknown` reward binding applies only when
  the verifier produced no result; an agent timeout whose verifier still
  ran and scored keeps the normal reward path. An agent-timeout job whose
  pinned-runner stats carry an empty-reward eval entry attributed to the
  exception, and a job whose agent or verifier timed out while the runner
  wrote no eval stats, bind the same honest `Unknown`
  (`agent-timeout`/`verifier-timeout`) after the full identity checks.
  A binding `task-not-observed` failure (an honest setup refusal with a
  bound identity, such as a runtime-preflight transport failure after all
  retries) settles at the Infrastructure boundary, invalidating one trial
  while the run continues; every other binding token still means identity
  or provenance corruption and stays integrity-fatal.
- `opi-eval`: offline reports and plans derive facts from sealed evidence
  and typed rejection instead of miscounting or crashing. Offline report
  usage facts derive from actual known token measurements in the sealed
  trajectory instead of sealed-artifact presence (a manifest whose
  measurements are both unknown reconstructs an honest
  `usage-not-measured`), `run` completion and offline `report`
  reconstruction require every planned trial to have a valid graded
  outcome (a single-subject run whose only trials are invalid executions
  exits 1 and reconstructs incomplete, while valid native zero scores,
  benchmark-defined Agent timeouts and no-edge single-subject success stay
  completed), offline native comparisons validate sealed exact-client
  reasoning evidence for the first request and tool continuation, the
  Terminal-Bench 3.0 and DeepSWE production profiles bind their selected
  tasks' actual package bytes and lifecycle metadata, non-live connection
  admission parses the actual HTTP(S) URL host before applying the
  loopback allowlist, selection expansion checks trial cardinality,
  vector storage bounds and the declared `max_trials` before allocating
  trial records (oversized repetitions return a typed rejection instead of
  an allocation abort), and `regrade` resolves its requested fresh output
  through the canonical nearest existing ancestor before any effect and
  rejects outputs contained in the canonical run root, including through
  missing parent chains and symlink aliases.
- `opi-eval`: retained-evidence maintenance stays reviewable. Manual
  maintenance plans bind selections by inventory digest instead of
  embedding per-file inventories (plans shrink from 100+ MB to kilobytes),
  the reference scan skips staged upstream subtrees (`families/*/cache`,
  `builds`, `material/packages`) so oversized runner registries and
  uv-installed hardlinks no longer mark real evaluations permanently
  unreadable, and purge tolerates staged hardlinks and symlinks (archive
  still refuses them).
- `opi-eval`: the offline acceptance verifier follows sealed settlement
  truth. A benchmark-valid Agent non-zero exit or task timeout - including
  a timeout tail without a final AgentEnd after a completed provider/tool
  loop with its matching native task exception - counts among the required
  execution trials, while provider, authentication, infrastructure,
  preflight, undispatched and invalidly graded terminals remain ineligible.
  Real acceptance requires sealed live first-connection evidence and
  matching native client observations; internally consistent fixture
  trials and loopback control probes cannot establish it. Offline local
  acceptance checks the current Opi boolean thinking-level map and
  thinking block (including disabled thinking, and pi's omitted
  level map when reasoning is off) and interprets Opi's native
  `Anthropic` API value and pi's `anthropic-messages` value for the first
  connection, retaining exact provider/model and settled tool evidence
  requirements.
- `opi-eval`: execution-blocker repairs from the six-trial acceptance
  re-run (audit gap P19-AUD-001). (1) `build-agent-artifacts.sh` now builds
  the Opi agent for the host-arch musl target (statically linked,
  fail-closed check) so it loads inside pinned task environments whose
  glibc predates the build host's. (2) The binary-local native capture
  grammar admits the current product stream: the `session_header` opener
  (schema_version 2), session-level `AutoRetryStart`/`AutoRetryEnd`, the
  inner `QueueUpdate` envelope, and `--json-compact` delta events that
  legitimately omit the cumulative `partial` snapshot; private thinking
  deltas are still emptied. (3) `verify-local-acceptance.py` admits a
  graded `agent-outcome-not-success` terminal (clean agent-side stop such
  as a `max_tokens` end) exactly as the Companion's own settlement
  vocabulary does, and accepts per-family control blocks so acceptance
  runs whose primary sources prescribe different sampling/output protocols
  per benchmark stay verifiable. (4) The Harbor/Pier integrations copy the
  capture receipt out through the daemon in the mounted case: the live
  bind mount leaves the helper's files owned by the container user with
  mode 0600, unreadable to the host terminal observation until the
  runner's post-stop chown. The locked native-smoke CI contract and the
  integration test mounts were updated in lockstep.
- `opi-coding-agent`: API-key resolution on macOS hosts without a default
  keychain (headless CI runners, SSH sessions without an unlocked login
  keychain) classifies Security.framework `errSecNotAvailable`,
  `errSecNoSuchKeychain`, and `errSecNoDefaultKeychain` failures as a
  backend-unavailable condition and falls back to the configured
  `api_key_env` variable instead of failing with `invalid provider
  configuration`. Read-only, permission, and corrupt-keychain failures
  remain hard errors.
- `opi-eval`: the Companion compiles and its test suites run on Windows
  and macOS CI again. Windows and macOS clippy and test compilation was
  restored (platform-gated imports, safe process kill paths, a portable
  sessions temp root instead of `/dev/shm`), native-material admission
  resolves macOS `/var`-rooted temp layouts to their symlink-free form,
  pinned-corpus path keys normalize to Git separators on Windows, and the
  native-driver integration suite (which requires a Linux Docker engine,
  `/proc`, and the Unix single-link capture contract) is gated to Linux
  alongside platform-honest doctor observations.

## 0.8.2 - 2026-09-05

### Breaking Changes

- `opi-eval`: active schema identities now use the `opi-eval-*` namespace,
  invariant and diagnostic codes use `EVAL-*`, and scripts, workflows,
  fixtures, and tests use capability-oriented names. The former Phase 18
  identities are rejected without compatibility aliases because the crate is
  unpublished and remains a `0.x` Independent Companion.

### Changed

- `opi-eval`: helper, verifier, native-smoke, contract-test, and CI fixture
  scripts now live under `crates/opi-eval/scripts/`, so the Independent
  Companion owns its project-specific tooling.
- `opi-eval`: completed-delivery-only baselines, seam-matrix derivation,
  rollback checks, and CI attestation assets were retired from the active
  package. Durable generic experiment coverage remains in
  `tests/experiment_contract.rs`; completed delivery evidence remains under
  `docs/snapshots/` and in Git history.
- `opi-sandbox`: its package manifest template and package, smoke, and
  package-helper scripts now live under `crates/opi-sandbox/`, so the
  Independent Companion owns its project-specific tooling.

### Fixed

- `opi-agent`: a response ending with `length` rejects its entire tool-call
  batch before preflight, hooks, authorization or execution, including calls
  whose arguments happen to parse. Ordered error results preserve normal
  stop/queue/continuation behavior so later complete calls can execute.

- `opi-eval`: regrading now fails closed when the run's trials directory
  cannot be enumerated instead of publishing an empty verified report.
- `opi-eval`: reused trial identities are now rejected from their durable
  intent reservation before staging can change the existing trial tree.
- `opi-eval`: artifact-directory and
  report-output ancestor aliases before they can redirect writes outside the
  reserved boundary, durably synchronizes the containing directory before an
  intent proof is returned, validates the complete Opi evidence graph before
  accepting an imported trace, and stages behavior-equivalent native command
  helpers for Windows hermetic runs. The Windows smoke wrapper also emits
  portable SHA-256 evidence and BOM-free JSON without assuming PowerShell
  module autoloading, and downloaded native artifacts verify without assuming
  a `python3` launcher or POSIX host path separators.
- `opi-eval`: sealed trial bundles now carry the complete retained-byte
  closure. The pre-effect intent reservation names
  every artifact identity the sealed bundle must cover (the resolved
  experiment, the integrity record, the trajectory projection, the
  normalized expected output, the agent execution streams, and the
  authority ledger), sealing enforces reservation equality against the
  staged set plus exactly the declared produced native evidence and
  requires the expected output to exist, and verification additionally
  compares the manifest's intent/settlement with the durable sidecars,
  rejects unmanifested or missing files under the sealed artifact tree,
  and keeps re-verification read-only and byte-stable. Promised agent and
  verifier artifact reads fail closed instead of being silently skipped.
- `opi-eval`: Agent-owned failures stay in the graded Agent outcome
  class. A closed failure classification now
  separates the Agent's own non-zero exit, crash, and supervisor-budget
  timeout (scored Agent outcomes that dispatch the native verifier over
  the settled workspace and pair as comparable graded outcomes in the
  Agent success/failure denominator) from actual authority-boundary
  failures (spawn refusal, cancellation, adapter/evidence rejection, and
  infrastructure), which keep their mechanical transition stops. The
  authority ledger seals the observed Agent completion class with the
  transition evidence.
- `opi-eval`: a natural child exit no longer abandons descendant
  processes. The supervisor retains whether each
  bounded stream reached EOF, probes the process-tree guard after the
  direct child exits, and runs the same terminate/verify sequence when
  descendants or inherited-pipe holders remain; cleanup is reported as
  not required only after observed tree emptiness, otherwise as verified
  tree termination or a typed termination failure.
- `opi-eval`: the DeepSWE native reward contract is enforced. Pier
  job-aggregate import accepts finite benchmark-defined
  score breakdowns such as F2P and P2P while rejecting authoritative
  `reward` values outside the native zero-or-one domain (negative, above-one,
  and fractional values) before any `u64` conversion. The DeepSWE upstream
  oracle preflight now requires an explicitly known positive native `reward`
  metric: a zero-reward reference solution no longer admits a task.
- `opi-eval`: every durable trial intent binds to the comparison edge
  that owns it. The runner resolves each trial's
  unique owning edge from its subject and the counterpart trial declared
  in the same task and group before any process effect, uses that edge as
  the durable `PairIdentity`, and rejects zero or ambiguous owners before
  Agent dispatch instead of defaulting every intent to the first declared
  edge.
- `opi-eval`: sealed provenance names the producer of every retained
  byte. Verifier stdout/stderr and the imported
  native grader reports are staged under a distinct grader source
  identity derived from the pinned benchmark adapter identity instead of
  the Agent's, and the offline headline selection requires the native
  grader role and grader source together instead of recovering the report
  by key suffix; source-role mismatches yield no headline.
- `crates/opi-eval/scripts/native-smoke.sh`: the conformance-rerun stage receipt
  reports the actual executed case count. The
  count derives from a loop counter incremented after each successful
  case instead of a hardcoded literal, and the CI contract test compares
  the declared case list with the counter path so the receipt can never
  drift from the executed set.
- `opi-eval`: offline reports derive only from verified sealed inputs.
  `opi-eval report` reconstructs trial views,
  pair coverage, integrity provenance, native rewards, and diagnostics
  from the sealed control evidence, trajectory, authority ledger, and
  manifest identities inside verified bundles - never from the mutable
  outer run report or trial receipts. A covered-byte mutation or a
  sealed-input parse failure returns a typed non-published outcome with
  a non-zero exit instead of publishing, and the `--out` path is opened
  with create-new semantics outside the run root so neither sealed
  bytes nor a prior report can be replaced.

## 0.8.1 - 2026-08-25

### Breaking Changes

- `opi-ai`: provider dispatch is collection-owned. `ProviderCollection` is the
  route/auth seam — `prepare_call` resolves one canonical `provider:model`
  route and authentication once per logical call, and every retry attempt
  reuses the frozen route/request/auth through the opaque
  `PreparedProviderCall::start_attempt`; `PreparedRoute` and redacted auth
  provenance are its only public facts. The
  `Provider::stream(Request)` entry, the `SharedProvider` wrapper, and the
  metadata-only `MetadataProvider` construction path were removed; unknown,
  ambiguous, unauthenticated, refresh-failed, fallback-disallowed, or
  wire-incompatible selections fail with typed errors before model HTTP
  dispatch, without silent provider or credential fallback.
- `opi-ai`: arbitrary `ProviderErrorSummary` construction and string
  conversions were closed; public callers must use `redacted()`,
  `authentication_rejected()`, or `from_untrusted(...)`, which does not retain its
  payload. The unreachable `CollectionError::AuthNotConfigured` variant was
  removed in favor of the dispatchable-route and typed provider-auth errors.
- `opi-agent`: `Agent` owns one durable atomic `NextTurnState` (context,
  provider:model, thinking, max_tokens, temperature). Request-transform hooks
  finish before collection-owned schema/capability validation; after a turn,
  the fixed order is prepare the candidate → validate it as a unit → atomically
  apply it → run the stop hook against applied state → poll queues. The
  append-only `AgentLoopTurnUpdate`, the unused
  `AgentHarness`/`HarnessRuntimeConfig`
  state owner, `SharedProvider`, and `Agent::add_tool` were removed. Tools are
  registered as immutable trusted `RegisteredTool`s and every execution passes
  a mandatory `ToolAuthorizer`; the pre-tool hook's authorization-suggesting
  `Allow` grant was renamed `Continue`, and missing, failed, expired, stale, or
  forged authority yields zero executions. Piecemeal Agent state setters, the
  generic state bag, and unused phase/snapshot/session harness owners were
  removed; `ContentDigest::from_hex` now validates canonical SHA-256 text.
- `opi-agent`: the storage-shaped core `TraceSink`/`TraceCollector` contract
  was superseded by the product-neutral evidence lifecycle
  (`EvidenceSink`/`EvidenceRecorder`, opaque run/turn/call and compaction
  identities, `EvidenceHealth`, immutable `FinalizedManifest`). `EvidenceSink`
  now separates `finalize_run` from `abandon_run`; evidence and manifest route,
  auth, tool, session, measurement, and terminal facts are typed, and product
  identity/reference strings use validated opaque wrappers. `RunId` is now a
  process-independent UUIDv7 serialized and parsed as its canonical hyphenated
  string rather than a numeric/process-local identity.
- `opi-agent`: `Agent` run operations and the low-level `agent_loop` now return the
  must-use `AgentRunResult` and `AgentLoopResult`, preserving actual state,
  owning error, terminal outcome, and evidence health on failure. Post-loop
  evidence/compaction uses `AgentRunLifecyclePhase`, `PendingCompaction`, and
  explicit finalize/abandon APIs. Preflight cancellation uses opaque
  `ArmedAgentRun` generations; a stale or foreign generation returns typed
  `AgentError::InvalidArmedRun`.
- `opi-coding-agent`: legacy bare-model session routes normalize only when the
  dispatchable collection proves exactly one route; missing or ambiguous
  routes keep the configured model and report typed remediation instead of
  guessing from the active provider. Direct model selection
  (`set_model`/`set_model_validated`, RPC `set_model`, and CLI startup
  validation) now applies the same unique-dispatchable-route proof to bare
  input, and a model change that cannot be dispatched is rejected before any
  durable `model_change` entry is written.
- `opi-agent`: an in-band provider stream `Error` terminal (for example an
  SSE `error` event) now fails the run with the typed, non-retryable stream
  failure instead of completing the turn as a normal assistant message; the
  partial assistant message remains visible in the run's event stream.
- `opi-ai`: every concrete adapter validates the prepared `AuthScheme` at its
  wire boundary and rejects a mismatched scheme with a typed configuration
  error before attaching the secret, instead of attaching it unconditionally.
- `opi-agent`: `InMemoryEvidenceSink` fails closed with the typed evidence
  error when `emit`/`finalize_artifact` run before `setup`, matching the file
  adapter's lifecycle contract.
- `opi-agent`: the closed decision enums `AuthorizationDecision` and
  `TerminalOutcome` are no longer `#[non_exhaustive]`.
- `opi-coding-agent`: the speculative `register_extension_tools` seam and the
  discarded `_extension_tools` parameter of `register_product_tools` were
  removed (extension tools remain excluded from product registration).
- `opi-coding-agent`: `NonInteractiveRunner::cancel` was replaced by
  `cancel_token(&mut self)`, which arms the next run generation and returns its
  clonable cancellation token before `run*` takes the mutable borrow.
- `opi-coding-agent`: conflicting registered project-trust resolver votes now
  combine fail-closed: `Deny` dominates `Trust` regardless of registration
  order, while `Trust` applies only when no resolver denies. This changes the
  previous first-decided short-circuit behavior for embedders.
- `opi-implement`: live implementation ledgers now require schema v2 and no
  longer auto-migrate a schema-v1 ledger. Historical v1 snapshots remain
  immutable audit evidence; preserve any still-live v1 file, then initialize
  and review a fresh v2 graph from the registered sources before resuming
  implementation.
- Project assurance: a Phase audit set is now a live index of independently
  installed `audit.<reviewer-id>.<model-id>.*` report groups rather than one
  atomically published generation. `audit.index.json` moves to schema 2 and
  member metadata to schema 3: there is no generation identity or set-wide
  head, each member carries its own committed `audit_head`, reports install
  immediately via `assurance_set.py complete`, and a reviewer/model re-run
  replaces its own entry while archiving the superseded run under
  `history/<audit-run-id>/`. Remediation consumes every indexed finding and
  binds approval to the exact `audit.index.json` digest alone. Legacy schema-1
  sets require the one-time `assurance_set.py migrate` and are not dual-read;
  assurance paths are now pinned to LF in `.gitattributes` so byte digests
  survive fresh checkouts.

### Added

- `opi-ai`: `ProviderCollection` with per-call `prepare_call` route/auth
  preparation, `AuthResolver`/`ResolvedAuth` carrying non-secret provenance,
  and typed collection failures (`RouteNotDispatchable`,
  `RequestRouteMismatch`, `CallCancelled`, `CredentialTerminated`,
  `AttemptAlreadyActive`, ...).
- `opi-agent`: the product-neutral evidence module — `EvidenceSink` lifecycle,
  `EvidenceRecorder`, `EvidenceRecord` with call-graph correlation,
  `EvidenceHealth`, and no-op/in-memory adapters; trusted tool registration
  (`RegisteredTool`/`ToolRegistry`) with mandatory `ToolAuthorizer`
  authorization and validated opaque capability identities.
- `opi-coding-agent`: opt-in `--trace <PATH>` evidence capture in interactive,
  non-interactive text, JSON, and RPC modes. The path is a capture root; every
  run receives an immutable child directory containing `evidence.jsonl` and
  `manifest.json`. Also added eager
  multi-route dispatch with cross-provider switching without harness
  reconstruction; `FileEvidenceSink` with fail-closed setup.
  The Reference Product owns digest-addressed `EffectiveUserPolicy` snapshots,
  fixed CLI/SDK/RPC assembly identities, built-in capability identities, and
  active-session bindings; Agent Core supplies only product-neutral validated
  types and enforcement mechanisms.

### Changed

- Project skills now derive executable graphs only from reviewed, registered
  Phase delivery sources instead of legacy parent-spec roadmap sections; graph
  approval, local commit authorization, historical audit evidence, eval
  uncertainty, and remediation formatting have explicit fail-closed
  boundaries. The bilingual skill manual now includes lifecycle return loops,
  per-skill side effects, and artifact ownership, and every skill uses the
  standard `SKILL.md` entry name. A bare `opi-document` invocation now performs
  an implementation-backed audit of every maintained current-product README;
  targeted and version-bump scopes are explicit.
- The Reference Product runs one coherent runtime: interactive,
  non-interactive/print, JSON/NDJSON, and RPC entry points expose equivalent
  route, authority, cancellation, and evidence semantics over the same
  `CodingHarness` (CI selects the same hermetic Phase 17 acceptance on Linux,
  macOS, and Windows).
- Legacy serialize-only trace files remain opaque and byte-identical at their
  existing locations; new-schema evidence never overwrites, rewrites,
  upgrades, down-converts, or deletes them, and sessions are never rewritten
  by load, normalization, resume, or fork.
- `opi-agent` session writers now emit envelope-based version 2 files whose
  headers carry a required exact runtime-input binding; `SessionHeader`
  construction requires that binding. The Reference Product reads genuine
  version 1 sessions without modifying their source bytes, then resumes or
  forks only after uniquely normalizing their recorded route into a parented,
  exactly bound version 2 child. Corrupt, unsupported, missing, or ambiguous
  legacy inputs fail closed before execution; only version 2 sessions are
  mutable writers.
- `opi-ai`: Bedrock HTTP streaming now treats decoder residue at EOF as a
  typed stream failure rather than flushing a pending successful terminal
  event.
- Evidence manifests are bound to the current run's exact system prompt,
  trusted tool schemas, route, inference budget, active session branch, and
  terminal provider response. Setup, requested-session reopen, and durable
  finalization failures now remain visible instead of silently degrading.
- `opi-agent`: an `after_tool_call` replacement now changes only the emitted
  and persisted presentation result; tool evidence retains the original
  lower-boundary execution outcome instead of allowing a hook to rewrite what
  actually executed.
- The public event boundary (`AgentEvent::redacted_for_public`) scrubs
  recognized credential patterns (API-key, bearer, and JWT shapes) from user
  message content, tool-result content, and terminal tool results before they
  reach NDJSON/RPC output; ordinary conversation content continues to echo
  verbatim.
- An invalid configured startup model (for example a mistyped
  `--model provider:model` id) exits at CLI startup with a typed diagnostic
  instead of panicking during harness construction.
- Providers configured with broken non-secret configuration (for example an
  invalid proxy URL) that are skipped during eager extra-route construction
  now surface a redacted startup diagnostic naming the dropped provider,
  instead of degrading later model switches to unexplained unknown-model
  errors.
- The RPC `session_info` `tree_read_error` field is summary-redacted like its
  sibling fields, so raw session-file paths no longer cross the RPC boundary.
- Local `credential_process` execution is granted a ten-second budget
  (previously three): cold shell startup on a loaded Windows host alone can
  exceed three seconds, turning healthy credential helpers into spurious
  typed timeouts. Cancelling a credential process now terminates its whole
  process group with a portable kill invocation, so descendants no longer
  survive resolver cancellation on Linux.

## 0.8.0 - 2026-08-12

No runtime behavior changes. This release consolidates the Opi specification
and project documentation, refreshes alignment and research evidence, and
refines repository governance plus the documentation-contract checker.
Published crate APIs and the `opi` CLI are unchanged from 0.7.3.

### Changed

- `docs/opi-spec.md` and `docs/opi-spec.zh.md` were substantially rewritten to
  tighten durable product direction, architecture invariants, and admission
  gates; `docs/CONTEXT.md` domain language was refreshed to match.
- README (EN/ZH) and the six crate-level READMEs (EN/ZH): top-level technical
  direction and project guidance streamlined; version surfaces updated to 0.8.0.
- Completed phase design documents were archived under `docs/snapshots/` and
  stale `docs/superpowers/specs/` references pruned; in-source design references
  in `opi-coding-agent` execution tests were repointed to their snapshot
  locations.

### Added

- Repository skill/workflow governance: admission and fidelity contracts across
  the `opi-audit`, `opi-eval`, and `opi-implement` skills.
- `scripts/opi-doc-check.py` was extended with new source-derived contracts,
  accompanied by a `scripts/test_opi_doc_check.py` test suite.
- Fresh pi 0.84.1 realignment reports (`docs/realign/2026-08-11-opi-vs-pi.*`,
  EN/ZH) and refreshed outward research evidence under `docs/research/`.

## 0.7.3 - 2026-08-11

> Note: on macOS, `opi-sandbox`'s seatbelt substrate does not propagate the
> invocation `TMPDIR` to the sandboxed target in this release, so sandboxed
> commands whose temp writes rely on `$TMPDIR` (for example `mktemp`) are
> denied on macOS. Linux is unaffected. Tracked for a follow-up.

### Breaking Changes

- `opi-coding-agent` 0.x API: `BashResult` now carries the required typed
  `BashOperationContext` and `BashExecutionContract` instead of duplicating
  exit, signal, truncation, and effective-contract state through flat fields
  and a magic diagnostic payload.
- `opi-coding-agent` 0.x: the built-in Phase 15 native sandbox is removed from
  the Opi core (migration 16.16.1). The `[sandbox]` section and the `--sandbox` /
  `--sandbox-require` flags are rejected in core without compatibility aliases;
  native restriction and its helper/capability-selection code now live in the
  standalone `opi-sandbox` package, selected through the `command.execute`
  execution backend instead of `[sandbox] mode`. Project-local
  executable/process package contributions are rejected; install globally,
  review, and enable.
- The workspace adds the `opi-protocol` and `opi-sandbox` crates (lockstep
  workspace version). `opi-protocol` owns only the versioned
  `command-execution-jsonl-v1` execution protocol; `opi-sandbox` depends on
  `opi-protocol` plus standalone dependencies and publishes Linux/macOS
  archives only (no official Windows artifact).

### Added

- `opi-coding-agent`: the `command.execute` capability with a Minimal Runtime
  default. The model-callable `bash` tool runs directly through the built-in
  `local` backend unless `[execution] strategy = "fixed"|"rules"|"model"` and
  user permission policy (`deny`/`ask`/`allow`; project layers cannot set
  `[execution.permissions]`) select an installed external adapter. Installed,
  Trusted, Enabled, Selected, and Permitted are five independent lifecycle
  gates implemented by `opi package add/remove/list/doctor` and
  `PackageActivationStore`. Once an external adapter is selected, failure is
  fail-closed and never falls back to local execution.
- `opi-coding-agent`: 14 stable redacted `ExecutionFailure` codes (for example
  `package_not_installed`, `permission_required`, `protocol_violation`) with
  distinct actionable remediation, surfaced with consistent redaction across
  text, NDJSON, RPC, and interactive outputs; `package doctor` and `opi doctor`
  emit their own stable doctor-local codes (`doctor_package_exec_lifecycle` /
  `doctor_package_exec_drift`) for execution-package lifecycle and drift.
- `opi-protocol`: versioned `command.execute` protocol types, bounded codecs,
  JSON schemas, and shared fixtures under the `command-execution-jsonl-v1` wire
  identity.
- `opi-sandbox`: a standalone library SDK (`SandboxPolicy`, `SandboxRequest`,
  `SandboxRunner`, `SandboxEvent`/`SandboxResult`) and human CLI
  (`opi-sandbox run`, `opi-sandbox backend --stdio`, `opi-sandbox doctor
  --json`) for L0 process-tree supervision and Linux/macOS workspace-write
  restriction, reusable without Opi and with no Opi configuration, session, or
  package dependency.
- `opi-sandbox` native guarantees: Linux uses Landlock filesystem-mutation
  restriction plus a fixed seccomp danger blocklist (with `network = deny`
  new-socket/TCP restrictions); macOS uses `sandbox-exec` with host
  reads/execution allowed and writes confined to the workspace and invocation
  temporary roots; Windows Job Objects provide L0 supervision only, with no
  official `opi-sandbox` artifact.

### Changed

- `opi-coding-agent`: command execution now reports the selected backend's
  effective placement and guarantee (`local` reports `supervised`,
  `opi-sandbox` reports `restricted`) after setup succeeds; adapter identity
  alone never establishes a guarantee. L0 subprocess-tree supervision remains
  in core for both local and external adapter processes.

### Fixed

- `opi-coding-agent` / `opi-agent`: routed timeout completions retain the stable
  `execution_timed_out` diagnostic and remediation, while public tool events
  replace backend-authored diagnostic prose and redact exact invocation values
  from adapter-reported execution-contract text.
- `opi-coding-agent`: package trust and release verification now hash the exact
  `package.toml` bytes, so line-ending-only manifest drift invalidates the lock
  consistently across activation, resolution, packaging, and artifact audit.
- `opi-protocol` / `opi-sandbox`: completed-frame semantics and diagnostic
  framing bounds are validated before encoding; protocol writes are bounded by
  the invocation deadline; and owner-death/teardown paths retain one process
  owner through confirmed tree cleanup.
- `opi-sandbox`: standalone CLI and protocol-backend runs now stream complete
  stdout/stderr bytes through bounded backpressure while retaining bounded SDK
  terminal previews, so successful output beyond 1 MiB is neither dropped nor
  duplicated.
- `opi-coding-agent`: routed bash execution now shares the local recoverable
  preview/full-output policy, text and TUI modes preserve redacted tool failure
  diagnostics after provider recovery, and exact-cap CRLF protocol frames are
  accepted.
- `opi-coding-agent`: protocol teardown now gives unconfirmed tree termination,
  child reap, or stderr drain precedence over the original failure; macOS
  production builds also receive their required `tempfile` dependency.
- `opi-sandbox`: added fail-closed macOS missing/unusable-helper coverage and an
  isolated Linux Landlock network test seam, and updated Linux policy decoding
  for the declared toolchain's lint gate.

### Non-Goals (Phase 16)

- No Docker/VM/SSH/Gondolin or remote adapters; no routing of file,
  navigation, or other built-in tools; no core-tool shadowing by extensions;
  no universal extension protocol or migration of `opi-extension-jsonl-v1`,
  RPC, NDJSON, or trace envelopes; no dynamic native-library loading; no
  composing multiple adapters for one invocation; no host-read or
  environment-variable confidentiality; no sandboxing of the extension process;
  no publisher authentication; no project-local executable contributions; no
  Windows AppContainer or restricted-token restriction; and no preserving of
  unreleased Phase 15 sandbox configuration aliases.

## 0.7.2 - 2026-07-31

### Breaking Changes

- `opi-tui` 0.x API: `AppState` gains the `AwaitingTrust(AwaitingTrustState)` variant for the Phase 15 interactive project-trust prompt. The enum remains exhaustive, so downstream exhaustive matches on `AppState` must add the arm. Because `AwaitingTrustState` owns a oneshot sender, `AppState` no longer implements `Copy`, `Clone`, `PartialEq`, or `Eq`; use the copyable `AppStatus` projection when only display state is needed.
- `opi-coding-agent` 0.x API: sandbox diagnostic constructors now accept the closed `SandboxReason` enum instead of arbitrary string-like reasons, and `LocalFileOperations::new` now requires a workspace-root path (the zero-sized `Default` implementation was removed) so workspace operations can remain capability-relative.

### Added

- `opi-coding-agent`: OS-native subprocess-tree sandbox for `bash` as opt-in defense-in-depth (explicitly not a security boundary). An always-on L0 baseline (`process_group(0)` on Unix, a kill-on-close Job Object on Windows) ships in every mode; `[sandbox] mode = "strict"` (default `off`) opts into L1/L2/L3 layers and `require` (default `false`) selects fail-closed vs the default fail-open-with-diagnostic policy. Linux strict L2 is a narrowed seccomp new-socket creation gate (denies `socket(AF_INET/AF_INET6/AF_NETLINK)`, preserves `AF_UNIX`) plus Landlock ABI 4 (Linux 6.7+) TCP bind/connect; on ABI 1-3 fail-open retains the socket gate and reports the missing TCP capability, while `require = true` fails before spawn. It has a fixed L3 danger blocklist and explicit inherited-fd/non-TCP/`io_uring` residuals (no claim of complete network isolation); macOS probes and launches `/usr/bin/sandbox-exec`; Windows is L0-only and `strict` degrades to L0. CLI: `--sandbox off|strict`, `--sandbox-require`; additive `opi.sandbox.degraded` and `opi.sandbox.unavailable` diagnostics carry a redacted `{ layer, reason }` payload.
- `opi-coding-agent`: per-tool `Operations` seam — `FileOperations` and `BashOperations` traits with `Arc<dyn>` constructor injection into `read`/`write`/`edit`/`bash`, layered below `PathPolicy` (which stays the pre-flight). The sandbox lives inside local `LocalBashOperations::exec`; the shipped `LocalFileOperations` performs workspace reads, metadata, directory creation, staging, and rename relative to a held workspace-root capability, while explicitly allowed external reads remain ambient. Nav tools (`grep`/`find`/`ls`/`glob`) take no Operations handle. The seam ships local impls only (no SSH/container remote backends). The production-path `sandbox.rs`, `tool/operations.rs`, and `sandbox/windows.rs` modules retain `#![forbid(unsafe_code)]`.
- `opi-coding-agent`: project-trust gate that gates *loading* (not tool execution) of project-local resources. `ProjectTrustStore` is a flat `Map<canonical_path, bool>` at `{user_config_dir}/trust.json` (`%APPDATA%\opi\trust.json` / `~/.config/opi/trust.json`), consulted once at startup before `discover_resources`. An untrusted project skips its `.opi/config.toml`, `.opi/{skills,fragments,themes,extensions}`, project-scope `.opi/packages.toml` adapter declarations (so native adapter children never start), and project `AGENTS.md`/`CLAUDE.md` system-prompt injection (a deliberate pi divergence; the files remain `read`-able). CLI: `--trust` / `--no-trust`; `[defaults] default_project_trust = "ask"|"always"|"never"` (default `ask`, global-only). Trust resolvers register through an explicit embedder-only `ProjectTrustResolverRegistry`; the standard CLI ships an empty registry, has no CLI `-e`, no native resolver loading, no built-in `/trust`, and no live mid-session trust mutation or project-resource reload.
- `opi-tui`: `AppState::AwaitingTrust` variant and trust prompt widget for the interactive project-trust ask (Trust / Trust-parent / Trust-session / Deny / Deny-session).

### Changed

- `opi-coding-agent`: configuration resolution stages user, project, explicit, environment, and CLI layers so project config stays unread until trust authorization, canonical precedence is preserved, and custom providers validate only after the final merge. `doctor` and `--list-models` now use the same headless trust preflight while explicit `--config` remains user-authorized.

### Fixed

- `opi-coding-agent`: hardened workspace file operations against ancestor symlink/junction swaps after path-policy checks, made atomic staging and bash capture spills cancellation-safe through RAII cleanup, and added rejected-path zero-backend-call coverage for Write/Edit.
- `opi-coding-agent`: retained Linux's independent seccomp socket gate below Landlock ABI 4, restricted seccomp engagement to verified x86_64/aarch64 targets, bound macOS launch to the probed absolute helper, and made sandbox diagnostic reasons closed and redaction-safe.
- `opi-coding-agent` / `opi-tui`: omit Trust-parent at filesystem roots and reject forged root-level selections instead of returning an unpersisted trusted decision.

## 0.7.1 - 2026-07-24

### Breaking Changes

- Raised the workspace Minimum Supported Rust Version (MSRV) from 1.85 to 1.97 (`rust-version` in `[workspace.package]`, inherited by all crates). Builds now require Rust 1.97 or newer; the workspace remains on edition 2024.
- `opi-ai` 0.x API: `Request` adds `timeout`, `extra_headers`, `cache_retention`, and `session_id`; `Provider` adds the object-safe `refresh_models` and `replace_model_catalog` methods; `ModelInfo` replaces flattened capability fields with one nested `ModelCapabilities` and adds exact `WireApi`, thinking-map, wire-compatibility, and pricing metadata; `Usage.cache_write_1h_tokens` and `Usage.reasoning_tokens` are corrected from `u32` to `Option<u64>` so absent and explicit zero remain distinct; and `CostBreakdown` removes the separate `cache_write_1h_cost` field. Downstream struct literals and custom provider implementations must be updated.
- The development OAuth provider ids `copilot` and `codex` are replaced by canonical `github-copilot` and `openai-codex`. There is intentionally no config alias or keychain credential migration; affected users must log in again with the canonical id.
- `opi-agent` 0.x API: `AgentError` gains an `AccountIdMissing { provider_id }` variant for provider credentials that are present but lack a required account id (e.g. an OpenAI Codex token without `chatgpt_account_id`). The enum is exhaustive, so downstream exhaustive matches on `AgentError` must add the arm. JSON, RPC, and text modes surface it as a typed `/login <provider>` remediation with an AuthFailure exit code, distinct from `CredentialRevoked`.

### Added

- `opi-ai`: IO-free, object-safe `CredentialStore`, `Credential`, `CredentialSource`, `OAuthProvider`, `OAuthCredential`, `LoginPresenter`, `AuthResolver`, and `ResolvedAuth` contracts; secret-free `AuthDescriptor::StoreCredential`; and typed non-retryable `CredentialNeeded` / `CredentialRevoked` provider failures.
- `opi-coding-agent`: OS-keychain credential persistence using Windows Credential Manager, macOS Keychain Services, or Freedesktop Secret Service across the six release targets, with env API-key fallback, cross-process mutation locking, redacted doctor/model-list probes, and explicit interactive `/login <provider>` / `/logout <provider>` flows for Anthropic Browser PKCE, GitHub Copilot Device Code, and OpenAI Codex Browser (default) or Device Code. Device-code flows present the public device code and never request manual paste-back. Non-interactive, JSON, and RPC modes report provider-specific `/login` remediation without constructing a presenter or starting OAuth.
- `opi-ai` / `opi-agent` / `opi-coding-agent`: request timeout and extra-header wire handling; production `session_id` propagation with provider-specific cache-affinity mappings; nested cache capabilities and Anthropic cache markers; and cache-write/reasoning usage accounting without double counting.
- `opi-ai`: deterministic atomic dynamic-model catalog replacement through `Provider::refresh_models` and `ProviderCollection::refresh`. This remains substrate-only with no production trigger.
- `opi-ai` / `opi-coding-agent`: public `ApiMappedProvider` and `[providers.custom.<id>]` multi-wire configuration, exact `WireApi` routing, model/provider API and base-URL precedence, wire-tagged compatibility metadata, thinking maps, model pricing tiers, shared lazy credentials, and reserved managed-header validation.
- `opi-coding-agent`: audited static pi-0.80.6 GitHub Copilot catalog mapped across Anthropic Messages, OpenAI Completions/Chat, and OpenAI Responses, plus the dedicated `openai-codex-responses` provider and audited OpenAI Codex catalog.

### Changed

- Concrete Anthropic, all three GitHub Copilot routes, and dedicated OpenAI Codex Responses re-resolve authentication inside every returned stream. TUI `/help` discovers `/login` and `/logout` through the production dispatcher; after a pre-output `CredentialNeeded`, only a successful explicit login for the same provider retries the same pending turn, exactly once and without a duplicate user message. Missing or revoked credentials never auto-login.
- `CostBreakdown` remains separate and `Copy` with four public lines (`input_cost`, `output_cost`, `cache_read_cost`, and `cache_write_cost`): the weighted one-hour cache-write subset is folded into `cache_write_cost`, reasoning stays inside `output_cost`, and neither subset is counted twice.

### Fixed

- `opi-ai`: enforce selected-model capability preflight on every public dispatch path, collect complete atomic refresh batches, apply Anthropic compatibility metadata, preserve initial Chat tool arguments, materialize mapped-provider catalog overrides, and calculate cumulative cost from exact `u64` totals.
- `opi-coding-agent`: reject ambiguous provider identities, preserve foreign process-default keyring ownership, strictly decode credential envelopes, serialize OAuth refresh against public writes, and exercise the production TUI event dispatcher in debug and release test profiles.
- `opi-coding-agent`: the `ls` and `find` tools and the `write`/`edit` details now relativize inside-workspace paths to match `read`, so absolute workspace roots no longer leak into model-visible tool output or NDJSON `details` (macOS `/var`→`/private/var` symlink previously defeated the relativization). Also fixes the macOS and Freedesktop keychain backends failing to compile on Unix (the `apple-native-keyring-store` `keychain` and `zbus-secret-service-keyring-store` `rt-tokio-crypto-rust` features were not enabled), which had left the Phase 14 credential stores unbuildable on Linux/macOS.

## 0.7.0 - 2026-07-09

### Added

- `opi-coding-agent`: `--json-compact` opt-in flag that makes streamed `text_delta` updates constant-size (~linear byte cost for long streamed turns). It omits the redundant `assistant_event.partial` snapshot and empties the cumulative text in `event.message` for those updates. Default `--json` output and `NDJSON_SCHEMA_VERSION = 2` are unchanged.
- Repository: `scripts/opi-artifact-audit.py` (plus `.ps1`/`.sh` wrappers) — a deterministic, network-free checker for saved runtime, NDJSON, session, provider, and browser evidence (workspace-root leakage, all-zero timestamps, provider-turn mismatch, duplicate `text_delta` partials, failure claims without preserved artifacts) — paired with an `opi-implement` Artifact Truthfulness Gate (Phase D.0a) requiring verified artifacts for runtime/CLI/NDJSON/session claims. Backed by a fixture-driven `opi-coding-agent` integration test.

### Changed

- `opi-coding-agent`: `session_summary` now includes `provider_turns` (provider request/response cycles, i.e. `TurnStart` events) alongside the existing `turns` (user prompt turns), so a tool-using prompt usually reports `provider_turns > turns`.
- `opi-release`: pre-flight checks are documented as deterministic and must not depend on live provider or dogfood runs; any cited dogfood evidence must already have passed the `opi-implement` Artifact Truthfulness Gate.
- Bumped the workspace version to `0.7.0` and refreshed the Phase 4 and Phase 6 specification-hash ledgers to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-ai` / `opi-coding-agent`: support custom OpenAI-compatible chat completions endpoint paths for providers whose `base_url` already includes a provider-specific API prefix (e.g. BigModel `/api/paas/v4/...`), configured via `chat_completions_path` on an `openai_compatible` profile. Default OpenAI/OpenRouter/Mistral behavior still posts to `/v1/chat/completions`.
- `opi-ai` / `opi-agent` / `opi-coding-agent`: populate runtime message timestamps instead of emitting `timestamp_ms: 0` for runtime-produced user, assistant, and tool result messages.
- `opi-coding-agent`: avoid exposing resolved absolute workspace paths in successful `read` tool output text and structured `details` for inside-workspace reads.

## 0.6.5 - 2026-07-06

### Added

- `opi-agent`: Phase 13 session entry model. The session journal gains typed, forward-compatible entries (`session_info`, `model_change`, `thinking_level_change`, `label`, `branch_summary`) layered additively on the v1 header, with no automatic migration. `SessionReader` now distinguishes unknown future entry types (reported via `CrashRecovery::unknown_count`) from genuine corruption, and treats a JSON object lacking a `type` field as corrupt. `CrashRecovery` is now a struct `{truncated_line, corrupt_count, unknown_count}`. `SessionFacade` gains `enqueue_*` metadata methods that parent entries to the active content tip without advancing it.
- `opi-agent`: reusable session context reconstruction. `reconstruct_context(&[SessionEntry], &CrashRecovery) -> ReconstructedContext` deterministically walks the active branch by `parent_id`, applies compaction and `branch_summary` entries at their documented positions, collects model/thinking/session_name/labels/extension_state metadata parented to the active chain, and emits diagnostics for missing parents plus forwarded corrupt/truncated/unknown observations.
- `opi-coding-agent`: interactive session metadata commands. `/name`, `/label`, `/unlabel`, and `/session info` (plus RPC `session_info`) append and read typed `session_info`/`label` entries through `SessionCoordinator` parented to the active content tip without advancing it; labels and `session_info` never enter provider context.
- `opi-coding-agent`: local session export. `opi --export-session <id-or-path> --format markdown|json [--output ...]` supports active-branch and full-tree scopes, include/exclude flags for tool output and thinking content, and a Phase 7 redaction mode (`summary|verbose|none`). Dispatch runs before config/provider construction (network-free); the source session is opened read-only and stays byte-for-byte identical on success and failure.
- `opi-coding-agent`: session/tree handoff metadata. RPC `session_info` now emits active-branch `entry_count` and a redacted `branch_summary`, plus a `branches[]` array of `{tip, summary, entry_count, depth, active}` built from `SessionTree`, so embedders can render branch/session pickers without re-parsing JSONL. `session_picker_items` prefixes the typed session name to the display line and appends the label set to metadata.
- `opi-agent` / `opi-coding-agent`: resume and fork re-apply recorded model and thinking metadata when provider-compatible (`ResumeInfo` carries `recorded_model`/`recorded_thinking`), and `--list-sessions --json` emits deterministic rows carrying name, labels, active branch, model, and thinking metadata reconstructed from the active branch.

### Changed

- `opi-coding-agent`: resume, fork, branch-select, non-interactive resume, and `--list-sessions` now route through `opi_agent::session_context::reconstruct_context` instead of a duplicated product-only JSONL walker, which is retired.
- Documented the session format/version policy (v1 header kept, additive entries, no automatic migration), the unknown-future-entry vs corrupt recovery split, and local export/redaction/sensitivity behavior in `docs/opi-spec.md` and the crate READMEs (EN + ZH), with Phase 13 non-goal guard tests pinning the boundaries.
- Bumped the workspace version to `0.6.5` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-agent` / `opi-coding-agent`: aligned active-branch reconstruction across resume, fork, export, and session metadata, including invalid leaf fallback, rootless pre-turn metadata, and legacy no-leaf multi-root sessions.
- `opi-agent` / `opi-coding-agent`: surfaced stale `Leaf` fallback diagnostics, preserved metadata parented through active metadata entries during fork/resume seeding, shared harness session-tree reads across branch/RPC paths, and kept verbose session exports from redacting non-secret path evidence.
- `opi-coding-agent`: persisted model and thinking changes before mutating runtime state, forwarded stored branch summaries to providers on resume, redacted branch summaries and custom export messages, and kept RPC/session diagnostics from duplicating or hiding recovery errors.
- `opi-coding-agent`: handled empty `/name`, `/label`, and `/unlabel` commands locally instead of forwarding them to the model; `--format md` is now accepted as a documented export alias.

## 0.6.4 - 2026-07-05

### Added

- `opi-ai`: provider error taxonomy. `ProviderError::category()` returns one of nine documented `ProviderErrorCategory` classes (`Auth`, `Config`, `Request`, `Network`, `RateLimit`, `Provider`, `Stream`, `Capability`, `Cancelled`), distinguishing local pre-request failures from provider 4xx/5xx responses. Provider-side error diagnostics are redacted before reaching public surfaces.
- `opi-ai`: OpenAI-compatible provider profiles accept a `CompatConfig` (per-model `ModelCompatOverride` for role and max-tokens overrides, plus custom `extra_headers`) so Azure/OpenRouter/Mistral-style profiles can carry documented compatibility flags.

### Changed

- `opi-ai`: OpenAI-compatible streaming usage now requests `stream_options.include_usage` when `usage_in_stream` is enabled, preserves usage updates from any streaming chunk, and records OpenAI Chat response IDs from any chunk carrying `id`, not only role chunks.
- `opi-agent`: retry diagnostics now distinguish exhausted retry budgets from retry suppression after partial provider output.
- `opi-ai` / `opi-coding-agent`: missing usage is tracked explicitly as unknown instead of known-zero usage; session cost summaries are omitted when any turn has unknown usage or when pricing is unknown.
- Bumped the workspace version to `0.6.4` and refreshed the Phase 4 and Phase 6 specification-hash ledgers to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-ai`: OpenAI Responses tool-call deltas and item completion now route by output item identity instead of the last observed tool call.
- `opi-ai`: OpenAI Responses text lifecycle events now report the actual content index when text follows a tool call.
- `opi-ai`: Bedrock HTTP streaming now flushes a pending terminal `Done` event when metadata is absent.
- `opi-agent`: provider-returned cancellations now surface as `AgentError::Cancelled`.
- `opi-agent`: compaction and session-persistence public events redact secret-looking error text.

## 0.6.3 - 2026-07-01

### Added

- `opi-agent`: the agent loop lifts each tool-owned `ToolDiagnostic` into a Phase 7 `Diagnostic` (per-cause `CODE_TOOL_*` code + structured context) and mirrors it as a diagnostic-linked trace record, instead of collapsing every tool failure to a generic `tool_execution_failed` carrying only the tool name. `bash` failure diagnostics (nonzero exit, timeout, cancellation) now carry operation context (exit_code/cancelled/timed_out/truncated; raw command omitted).
- `opi-agent`: `AgentEvent::ToolExecutionEnd` exposes a `diagnostics` array on the JSON/NDJSON and RPC output paths (additive, `skip_serializing_if` empty; old payloads round-trip via `#[serde(default)]`). The provider-facing `ToolResultMessage` is unchanged.

### Changed

- Clarified Phase 11 tool-result, event, and session metadata contracts in the normative specs, including public redaction boundaries for tool details and diagnostics.
- `opi-coding-agent`: exhausting `max_turns` with tools still pending now returns `AgentError::MaxTurnsExceeded` and emits a `agent_max_turns_exceeded` warning diagnostic + trace, instead of silently returning `Ok(messages)`. Non-interactive/RPC runs that previously exited `0` on turn-cap exhaustion now exit `RuntimeFailure` (`1`). Runs that exhaust without pending tools (e.g. steering-driven continuation, zero-turn runs) still complete normally.
- Bumped the workspace version to `0.6.3` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order.

### Fixed

- `opi-agent`: redacted command/path-sensitive tool metadata before public events and session persistence while preserving provider-facing tool result content.
- `opi-coding-agent`: hardened Phase 11 built-in tool behavior for read byte caps and line-ending metadata, write-to-directory diagnostics, bash process/temp reliability, and bounded navigation work with skipped-file diagnostics.
- `opi-coding-agent`: corrected CLI help and README descriptions for Phase 11 tool policy, `glob` framing, and unique-match edit behavior.
- `opi-ai`: provider wire-converters now preserve failed tool-result semantics instead of making failure indistinguishable from success. Anthropic emits native `is_error: true` on the `tool_result` content block; OpenAI Chat (incl. Azure/OpenRouter/Mistral via the shared adapter) and OpenAI Responses prefix a deterministic `[tool_error] ` marker to the tool-output string (neither API has a native error field, and Responses does not accept a client-set `status` on input items); Gemini (incl. Vertex) sets `error: true` inside the `functionResponse.response` Struct. Bedrock already used native `toolResult.status`. The `is_error: false` body is byte-identical to the pre-fix shape on every provider.
- `opi-agent`: the `MaxTurnsExceeded` classification (previously dead code) is now constructed and classified at runtime. Tool-failure diagnostics now surface the per-cause filesystem/error code from the 11.2 taxonomy rather than the single generic collapse.
- `opi-coding-agent`: marked the `#[cfg(unix)]` `getuid()` permission-skip-guard `extern "C"` blocks in the find/ls/read-write-edit-bash tests as `unsafe extern` so they compile under the edition-2024 enforcement of newer Rust stable.

## 0.6.2 - 2026-06-28

### Added

- `opi-ai`: provider collection and authentication seam — a dedicated collection/auth runtime that lets model/auth ownership move out of ad hoc provider construction.
- `opi-agent`: generic `AgentHarness` seam separating generic phase/snapshot/session orchestration from the product-specific coding harness.
- `opi-agent`: session repository/facade seam (`SessionFacade`, `SessionRepo`) giving richer session context a first-class entry point instead of CLI-only writes.

### Changed

- `opi-coding-agent`: provider construction centralized into `provider_factory`; when a configured HTTP proxy fails to build, provider construction and `--list-models` now surface `failed to build HTTP client with proxy config: <cause>` instead of the bare cause string (message-wording change only).
- `opi-coding-agent`: `CodingHarness` documented as a product wrapper over the generic `opi-agent` seams, with runtime hook boundaries documented (Phase 10 WS10.4).
- Added Phase 10 documentation guards and an exit-trace completeness gate, and refreshed the root and crate READMEs, `opi-spec`, and the pi alignment matrix for the post-Phase-10 current state.
- Bumped the workspace version to `0.6.2` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release publishes the publishable crates to both GitHub Releases and crates.io in dependency order; it is the first crates.io release since `0.5.4` (`0.6.0` and `0.6.1` were GitHub-only documentation/guard-test releases).

### Fixed

- Addressed Phase 10 audit findings across the centralized provider factory and surrounding documentation.

## 0.6.1 - 2026-06-25

### Added

- `opi-coding-agent`: Phase 9 pi 0.80.2 baseline documentation guard tests pin the durable alignment-matrix evidence baseline, the normative specification, and the pi alignment matrix against `.repo/pi-0.80.2` as the current studied upstream, keep the Phase 9-14 roadmap consistent across English and Chinese counterparts, and reject current-scope overclaims for deferred ecosystem breadth (OAuth parity, image generation, custom extension UI parity, npm/gallery, web/share, and pi session compatibility).

### Changed

- Recorded the Phase 9.1 alignment-matrix, Phase 9.2 normative-specification, and Phase 9.3 supplemental-design baseline evidence, and archived the opi-implement Phase 9 ledger snapshot.
- Bumped the workspace version to `0.6.1` and refreshed the Phase 4 specification-hash ledger to match the current `docs/opi-spec.md`.
- This release is published to GitHub only; crates.io publishing is intentionally skipped because it contains documentation and guard-test changes only.

## 0.6.0 - 2026-06-24

### Changed

- Realigned the implementation roadmap against the `pi` 0.80.2 evidence baseline, adding Phase 9 baseline realignment, Phase 10 architecture deepening, and refreshed Phase 11-14 planning documents.
- Updated the English and Chinese technical specification and pi alignment matrix to reflect the current roadmap and phase boundaries.
- This release is published to GitHub only; crates.io publishing is intentionally skipped because it contains planning and documentation changes only.

## 0.5.x (0.5.0 - 0.5.4, 2026-06-07 to 2026-06-24)

Phase 4 and its hardening: the extension system (lifecycle hooks, custom
tools, commands, messages, state), RPC JSONL mode with SDK embedding types,
progressive resource discovery, session branching (`--fork`, `/tree`,
`/fork`, `/clone`), streaming proxy, custom provider registration,
OpenAI-compatible provider profiles, and package manifest V2 process
adapters bridged into the extension API with example packages.
Observability landed with a shared `Diagnostic` vocabulary and structured
redacted diagnostics across provider, retry, tool, compaction,
session-recovery, and RPC paths, the `ProviderErrorCategory` taxonomy, an
unstable trace envelope (`--trace`, RPC `trace` command, NDJSON
`StartupDiagnostics`), `opi doctor`, and machine-readable RPC `error_code`s.
SDK schema 3, NDJSON schema 2; `opi-web-ui` was removed from the workspace.

## 0.4.x (0.4.0, 2026-06-02)

Phase 3: cloud provider expansion (AWS Bedrock, Azure OpenAI, Google Vertex
AI), HTTP/HTTPS proxy support, image input across the stack, `find` and `ls`
tools, fuzzy pickers, terminal image rendering, shell completions, and
AGENTS.md/CLAUDE.md context loading.

## 0.3.x (0.3.0, 2026-05-25)

Phase 2 hardening: six LLM providers with retry/backoff and cost tracking,
session v1 JSONL persistence, context compaction, configurable TUI
(keybindings, themes, DiffView), and `--json` NDJSON output mode.

## 0.2.x (0.2.0, 2026-05-22)

Phase 1 MVP: Anthropic-based coding assistant with six tools, basic TUI,
TOML config precedence, and mock-provider integration tests.

## 0.1.x (0.1.0 - 0.1.1, 2026-05-20)

Initial scaffolding: five-crate workspace under lockstep versioning with
placeholder APIs, the `opi-release` and `opi-implement` skills, CI
workflows, and the technical specification document.
