# Batch AI-UTL — transcripts, data contracts, message semantics (Dim 19)

Batch summary:
- Contracts measured: 30 records over pi `packages/ai/src/utils/{transcript,json-parse,streaming-json,text,validation,overflow,identity,uuid,string-enum,diagnostics,event-stream,retry,hash}.ts` + `types.ts` + `api/transform-messages.ts` + transcript-pinning tests, against opi `crates/opi-ai/src/{transcript.rs,message.rs,identity.rs,text_decode.rs,context.rs,replay.rs,mutation.rs,retry.rs,stream.rs,provider.rs,schema/}`.
- Verdict tallies: equivalent 11, partial 8, different 7, missing 2, opi-only 2 (definitions per `00-inputs.md`).
- Biggest gaps: no incremental streaming-JSON parser (`StreamingJsonParser`/`ToolCallArgumentsStream`), no `ClientIdentity`/User-Agent/attribution framework, tool-anchor split semantics diverge from pi 1.5.0's three-mode `resolveTranscriptTools`, canonical `ToolCall.arguments` is raw text (pi: parsed object), whole-call retry pattern lists drift from pi 1.5.0 (missing entries, no status-code boundary lookarounds).
- Note: Opi modules pin `.repo/pi-1.3.0` sources (`crates/opi-ai/src/transcript.rs:24-27`, `replay.rs:13`); several `different` verdicts are 1.3.0→1.5.0 upstream drift, not Opi invention.
- Uncertainties: schema-engine `unresolvedRefs` reject-at-place behavior not verified here (AI-UTL-21); adapter-level timing points for `duration_ms` not traced per adapter.

All pi paths relative to `.pi/pi-1.5.0/`, all Opi paths relative to repo root. Line numbers from this snapshot inspection.

## Records

### AI-UTL-01 — transcript normalize/replay core
- Dimension(s): 19 (transcript semantics)
- pi entry: `packages/ai/src/utils/transcript.ts:10-112` (public: `createInitialSystemMessage`, `normalizeContext`, `getInitialSystemMessage`, `withoutInitialSystemMessage`, `getCurrentTools`, `getCurrentSystemMessage`, `getCurrentSystemPrompt`, `collapseSystemMessages`); pinned by `packages/ai/test/system-message-replay.test.ts:33-96`
- Opi entry: `crates/opi-ai/src/transcript.rs:276-480` (`create_initial_system_message` :276, `normalize_context` :306, `get_current_tools` :344, `get_current_system_message` :364, `system_message_text` :413, `render_system_message_update` :428, `collapse_system_messages` :452); pinned by `crates/opi-ai/src/transcript.rs:1053-1259` (unit) and `crates/opi-ai/tests/transcript_contract.rs:370-494`
- Comparison: equivalent
- Observable difference: replay rules match — content appended and joined `"\n\n"` skipping empties; sections replaced in place / `None` deletes / delete-then-readd appends; removals-before-additions per message; tool resolution keeps first-addition order; replayed head keeps the FIRST system timestamp; empty shorthand stays empty; collapse is a fresh value. Two pinned boundary divergences (documented in Opi module docs `transcript.rs:28-33`): pi `SystemMessage.content` is `string | TextContent[]` (`types.ts:567`) while Opi is a plain `String` (`transcript.rs:78`); `normalize_context` is fallible in Opi (validation, see AI-UTL-07) where pi never throws.
- Evidence basis: source inspection + test inspection (both sides' replay tests assert the same fixtures: sections `a`→replaced, `b`→removed, `c`→added; `base\n\nalso do this` content join).
- Review outcome: self-reviewed
- Notes: Opi `get_current_system_message` timestamp line (`transcript.rs:371-372`) keeps the first system timestamp like pi's `timestamp ??=` (`transcript.ts:81`).

### AI-UTL-02 — sections serialization and duplicate-key semantics
- Dimension(s): 19 (data contract)
- pi entry: `packages/ai/src/types.ts:569-577` (`sections?: Record<string, string | null>`); `packages/ai/src/utils/transcript.ts:84-90` (Map replay)
- Opi entry: `crates/opi-ai/src/transcript.rs:73-150` (`SystemMessage.sections: Vec<SystemSection>`, ordered-map serde with custom `serialize_sections`/`deserialize_sections`); round-trip tests `transcript.rs:1039-1090`
- Comparison: equivalent
- Observable difference: persisted JSON shape identical (`{"a": "one", "b": null}`); duplicate raw keys keep first position + last value on deserialize in both (JS object assignment semantics; Opi implements it explicitly `transcript.rs:127-135`, and `serde_json` is built with `preserve_order`, root `Cargo.toml:36`). One residual divergence: pi's own docs warn that JSON objects reorder integer-like section names (`types.ts:574-576`); Opi's ordered map preserves them in position. Non-string section values are rejected by Opi (`transcript.rs:1090-1095`) where JS would carry whatever the parser allowed.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-03 — temporal projection (`resolveTranscript`)
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/transcript.ts:115-121` (`resolveTranscript(context, supportsMidConvoSystemMessages)` → native-or-collapse binary)
- Opi entry: `crates/opi-ai/src/transcript.rs:470-546,548-700` (`TranscriptProjection::{NativeOrCollapse,Strict}`, split `TemporalCapabilities {body, sections, tools}`, per-wire capability resolution `compile_request_projection` :597-700); tests `transcript.rs:1261-1280`, `tests/transcript_contract/provider_projection.rs`
- Comparison: partial
- Observable difference: with a plain boolean the behaviors coincide (native keeps system messages in place, else collapse to replayed head; collapse never rewrites canonical). Opi adds: (a) per-axis capabilities (body/sections/tools independently) and an `AddOnly` tool mode that rewrites later system messages' `tools_added` to the computed real delta (`transcript.rs:655-681`); (b) `Strict` rejection before any request is built; (c) `TemporalProjectionFacts`/`ProjectionMode::{Lossless,Native,Collapse}` records. pi has none of these; pi's "additions"/"changes" transports handle anchoring adapter-side via `resolveTranscriptTools` (AI-UTL-04).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed
- Notes: Opi's AddOnly rewrite mirrors pi 1.5.0's "cached prefix" intent but at a different seam.

### AI-UTL-04 — tool split between request field and in-place changes
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/transcript.ts:230-279` (`TranscriptToolSupport = "none"|"additions"|"changes"`, `resolveTranscriptTools` with `foldedSystemMessages`); pinned by `packages/ai/test/transcript-tool-changes.test.ts:337-352` ("resolves the top-level tool boundary per transport")
- Opi entry: `crates/opi-ai/src/transcript.rs:944-1003` (`has_non_additive_tool_changes`, `TranscriptTools {request_tools, anchors_additions}`, `resolve_transcript_tools(messages, supports_tool_additions: bool)`); tests `transcript.rs:1370-1390`
- Comparison: different
- Observable difference: pi 1.5.0 `"additions"` folds at `max(leading?1:0, lastNonAdditiveChange+1)` — the top-level list becomes the tool state at the LAST removal/redeclaration and every LATER addition stays anchored in place (test `transcript-tool-changes.test.ts:341-352`: after a removal + re-add, `requestTools=[base_tool, drop_tool]` with `foldedSystemMessages:3` and the next `late_tool` still anchored). Opi anchors only when the WHOLE history is additive; any removal/redeclaration sends the complete current set top-level and anchors nothing (`transcript.rs:983-996`). Opi also has no `"changes"` mode (empty top-level list, all declarations in place — pi's Anthropic inline-tools path, `transcript-tool-changes.test.ts:31-104`). Callers see different wire payloads: prefix-cache friendliness after removals differs.
- Evidence basis: source inspection + test inspection (pi payload-capture tests)
- Review outcome: self-reviewed
- Notes: pi comment cites Kimi K3 verified October 2026 (`transcript.ts:208-210`); Opi's model is the pi 1.3.0-era two-way split.

### AI-UTL-05 — deprecated `hasToolRedefinitions`
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/transcript.ts:180-198` (`@deprecated`, kept for API compatibility); exercised `packages/ai/test/system-message-replay.test.ts:88-102`
- Opi entry: absent; Opi module docs state the deliberate omission (`crates/opi-ai/src/transcript.rs:27-29` "upstream-deprecated `hasToolRedefinitions` is deliberately not ported"); absence search: `grep -n "has_tool_redefinitions" crates/opi-ai/src/transcript.rs` → no matches
- Comparison: different
- Observable difference: pi exports a deprecated predicate callers may still invoke; Opi callers must use `has_non_additive_tool_changes` (:944), which is strictly broader (any removal OR redeclaration; pi's deprecated helper is redeclaration-only).
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Layer H: pi marks this API deprecated-normative ("will be removed in a future release"), implemented.

### AI-UTL-06 — declaration equality, state changes, declared tools
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/transcript.ts:123-177` (`toToolDeclaration`, `declarationsEqual`, `getToolStateChanges`, `getDeclaredTools`); tests `system-message-replay.test.ts:70-87`
- Opi entry: `crates/opi-ai/src/transcript.rs:843-941` (`to_tool_declaration` identity clone, `declarations_equal` structural, `get_tool_state_changes`, `tool_state_change_message`, `get_declared_tools`); tests `transcript.rs:1300-1368`
- Comparison: equivalent
- Observable difference: changed definition = removal + addition; equal states produce no delta (Opi adds an explicit `is_empty`/`tool_state_change_message` returning `None`, pi callers check emptiness); first-declaration order kept. Opi documents one numeric boundary (`transcript.rs:30-33`): `serde_json` structural equality distinguishes `1` from `1.0` where JS canonical serialization would not — records an extra delta at worst. pi's `toToolDeclaration` strips executable fields (`execute`, `constrainedSampling: undefined`); Opi's `ToolDef` is already a pure declaration so the strip is an identity clone (`transcript.rs:851-859`), and `constrainedSampling: false` vs absent is distinguishable in both.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-07 — transcript admission validation and projection facts (Opi extension)
- Dimension(s): 19
- pi entry: none in `utils/transcript.ts` (no validation of tool declarations or section names at transcript level; validation happens per tool call in `utils/validation.ts`)
- Opi entry: `crates/opi-ai/src/transcript.rs:152-273` (`validate_system_message`, `append_system_change`, `TranscriptError::{UnsupportedTemporalFidelity,InvalidDeclaration,DuplicateSectionName}`, `ToolDeclarationError::{EmptyName,SchemaCompilation{..},DuplicateName}`), `transcript.rs:476-546` (`ProjectionOptions`, `SourcePrefix` sha256, `MessageRange`, `ProjectionMode`, `ProjectionFacts`); tests `tests/transcript_contract.rs:63-369,550-601`
- Comparison: opi-only
- Observable difference: Opi fail-closed admission rejects (before installation, transcript unchanged): empty/duplicate tool names, schemas that fail to compile under the private dialect engine, and exact duplicate section names within one programmatic message; deserialization of `TranscriptContext` runs the same validation (`transcript.rs:154-170`). `ProjectionFacts` (message count + sha256 of the canonical serialization, temporal + replay facts) is stamped onto assistant messages (`message.rs:37-39`) — pi has no equivalent provenance record.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed
- Notes: JS callers can build transcripts pi would only reject lazily at validation time; under the strict reproduction lens this is added behavior (safer, still `opi-only` by the record rules).

### AI-UTL-08 — message envelope / session JSON structure
- Dimension(s): 19, message semantics
- pi entry: `packages/ai/src/types.ts:555-659` (`Message = SystemMessage|UserMessage|AssistantMessage|ToolResultMessage`, `role` tags `system|user|assistant|toolResult`, `timestamp` ms); `packages/ai/test/message-types.test.ts` (details typing)
- Opi entry: `crates/opi-ai/src/message.rs:16-31` (`Message` serde tag `role`, renames `user|system|assistant|tool_result`), `UserMessage` :121-124, `SystemMessage` `transcript.rs:66-119`, `TranscriptContext` serde-transparent `transcript.rs:148-170`; round-trip test `transcript.rs:1477-1486`, `tests/transcript_contract.rs:673-706`
- Comparison: partial
- Observable difference: structural semantics align (role-tagged chronology, ms timestamps, transparent message-list context), but the persisted JSON differs in field spellings: `timestamp_ms` vs `timestamp`, `tool_result` vs `toolResult`, `tools_added`/`tools_removed` vs `toolsAdded`/`toolsRemoved`, `stop_reason`/`error_message` vs `stopReason`/`errorMessage` (Opi uses snake_case everywhere; pi camelCase). pi `UserMessage.content` accepts a bare string (`types.ts:580`); Opi requires a block vector (`message.rs:121-124`). Neither side has a session-file version field or migration inside this package (pi session persistence lives in pi-agent-core/coding-agent; Opi's `stored_chronologies_round_trip...` test pins backward parse of pre-cutover lists only).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-09 — AssistantMessage field set
- Dimension(s): message semantics
- pi entry: `packages/ai/src/types.ts:585-638` (`api, provider, model, responseModel, providerThinkingLevel, thinkingLevel, diagnostics, usage, stopReason, deferred, errorMessage, rawStopReason, endTurn, timestamp, durationMs`)
- Opi entry: `crates/opi-ai/src/message.rs:34-86` (`projection` [opi-only], content, api, provider, model, response_model, `wire` [opi-only], response_id, usage, stop_reason, raw_stop_reason, error_message, timestamp_ms, diagnostics, duration_ms, provider_thinking_level, deferred)
- Comparison: partial
- Observable difference: Opi omits pi's `endTurn` (`types.ts:624-627`; absence search `grep -rn "end_turn|endTurn" crates/opi-ai/src` → only wire stop-reason strings `anthropic.rs:981`, `bedrock/mod.rs:1356`) and `thinkingLevel` (pi's agent-loop requested level). Opi adds `wire` (exact-wire provenance for replay identity) and `projection` facts. `durationMs` exists on both but is measured differently (AI-UTL-25).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-UTL-10 — ToolResultMessage field set
- Dimension(s): message semantics
- pi entry: `packages/ai/src/types.ts:640-658` (`toolCallId, toolName, content, details?, usage?, nestedCalls?, isError, timestamp, durationMs?`; `NestedToolCalls` :610-637)
- Opi entry: `crates/opi-ai/src/message.rs:88-107` (`tool_call_id, tool_name, content, details, is_error, not_executed [opi-only], truncated [opi-only], timestamp_ms`)
- Comparison: partial
- Observable difference: Opi lacks tool-result `usage`, `durationMs`, and the whole `nestedCalls` bounded record of nested tool calls (absence search: `grep -rn "nested_calls|NestedToolCall" crates/opi-ai/src` → no matches). Opi adds trusted-boundary facts pi has no equivalent for: `not_executed: Option<ToolNotExecutedReason>` (`message.rs:96-119`) and `truncated` (`message.rs:94`).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-UTL-11 — Usage and cost contract
- Dimension(s): message semantics, data contract
- pi entry: `packages/ai/src/types.ts:459-487` (`Usage` with required `totalTokens: number` and a nested required `cost {input,output,cacheRead,cacheWrite,total,totalFromProvider?}`)
- Opi entry: `crates/opi-ai/src/stream.rs:101-146` (`Usage {input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cache_write_1h_tokens?, reasoning_tokens?, total_tokens?, service_tier?, effective_service_tier?, reported}`); cost computed separately via `calculate_cost`/`Pricing` (`stream.rs:395-420`)
- Comparison: different
- Observable difference: pi embeds a cost object in every assistant message and a required numeric `totalTokens`; Opi's usage carries no cost (catalog pricing is applied on demand), keeps `total_tokens` optional-and-separate (`stream.rs:120-125`), adds `service_tier`/`effective_service_tier` and a `reported` flag distinguishing unknown usage from zeros (pi has no `reported`; absent numbers are just absent). Persisted transcript JSON therefore differs materially in the usage block.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-UTL-12 — ToolCall arguments data contract
- Dimension(s): message semantics
- pi entry: `packages/ai/src/types.ts:444-457` (`arguments: JsonObject`, `argumentsError?: string`, `thoughtSignature?`, `namespace?`)
- Opi entry: `crates/opi-ai/src/message.rs:258-288` (`arguments: String` raw text, `arguments_error: Option<String>`, `signature: Option<String>`)
- Comparison: different
- Observable difference: pi canonicalizes parsed arguments as a JSON object at finalize time; Opi keeps the model-sent raw text (or repaired text) as the canonical value and never stores a parsed object. Opi has no `namespace` field (OpenAI Responses namespaced tools). Error text differs too (AI-UTL-15). This is the deepest divergence in the message data contract: consumers of persisted transcripts must parse Opi arguments themselves.
- Evidence basis: source inspection + test inspection (`message.rs:395-465`)
- Review outcome: self-reviewed
- Notes: Opi replay re-parses raw arguments and fail-closes unmarked malformed text (`replay.rs:471-478`).

### AI-UTL-13 — `repairJson`
- Dimension(s): 19 (JSON tolerance)
- pi entry: `packages/ai/src/utils/json-parse.ts:12-84` (escape control chars, double backslashes before invalid escapes, `\u` needs 4 hex digits, trailing backslash doubled); tests `packages/ai/test/json-parse.test.ts`
- Opi entry: `crates/opi-ai/src/message.rs:330-410` (`repair_json`, pinned); tests `message.rs` repair cases + `tests/public_utilities.rs:276-310`
- Comparison: equivalent
- Observable difference: same algorithm — outside strings verbatim; inside strings raw control chars escaped (`\b\f\n\r\t` or `\u00xx`), legal escapes and valid `\uXXXX` pass through, invalid escape → backslash doubled keeping the char literal, `\u` without 4 hex digits treated as invalid escape, trailing backslash doubled. Both operate on scalar iteration so results match for BMP and astral text.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-14 — streaming partial-JSON cascade
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/json-parse.ts:124-146` (`parseStreamingJson`: full parse → repaired full parse → `partial-json` partialParse(raw) → partialParse(repaired) → `{}`)
- Opi entry: `crates/opi-ai/src/message.rs:412-644` (`parse_streaming_json` with own `complete_partial_json` + `fix_dangling_token`); tests `tests/public_utilities.rs:311-391`
- Comparison: equivalent
- Observable difference: identical cascade order and never-fails contract; Opi reimplements partial completion (dangling quotes/brackets filled, truncated trailing token completed conservatively or dropped, junk after first complete top-level value ignored) instead of depending on `partial-json` (absence search: `grep -rn "partial-json" crates/opi-ai` → no matches). Edge-case parity of the completion heuristics vs the npm `partial-json` library was not differentially tested here (both sides' own tests pass on their own fixtures).
- Evidence basis: source inspection + test inspection
- Review outcome: unreviewed
- Notes: display-only in both (`message.rs:412-427` doc mirrors pi's "never a final-parse authority").

### AI-UTL-15 — final tool-argument parse
- Dimension(s): 19, message semantics
- pi entry: `packages/ai/src/utils/json-parse.ts:90-121` (`parseToolCallArguments`: `{}` + `argumentsError` on failure; error embeds `Received arguments:\n${json}` — the raw model text)
- Opi entry: `crates/opi-ai/src/message.rs:290-327` (`ToolCall::finalize_text_arguments`, public `finalize_tool_arguments` :317); tests `message.rs:395-465`, `tests/public_utilities.rs:392-454`
- Comparison: different
- Observable difference: three deliberate divergences. (1) Result shape: pi returns a parsed object + `{}` on failure; Opi returns owned raw/repaired text (keeps raw when malformed). (2) Error text: Opi uses the fixed redacted string `"final tool arguments were not a complete JSON object"` and never echoes model-sent text; pi embeds the full received arguments (used to report back to the model). (3) Opi keeps repaired text when only escapes/control chars were repaired; pi returns the parsed value of either raw or repaired. Callers that rely on showing the model its own malformed arguments cannot do so from Opi's transcript.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-16 — incremental streaming-JSON parser
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/streaming-json.ts:88-467` (`StreamingJsonParser.append/snapshot` with linear-time cost model, change reports `set`/`append`, 32-depth reporting cap; `ToolCallArgumentsStream` raw-text + partial-args + `replace()`/`finish()`); tests `packages/ai/test/streaming-json.test.ts`, `streaming-tool-arguments.test.ts`
- Opi entry: absent. Absence search: `grep -rn "StreamingJsonParser\|ToolCallArgumentsStream" crates/opi-ai/src crates/opi-ai/tests` → no matches. Opi adapters accumulate raw argument text per delta (`crates/opi-ai/src/anthropic.rs:816-882`) and re-run whole-text `parse_streaming_json` for display (`mistral.rs:1313`, `pi_messages.rs:1134`)
- Comparison: missing
- Observable difference: pi providers keep one linear-time parser per streaming call and hand out partial arguments without re-parsing; Opi re-parses the complete text per delta (quadratic in argument length for display) and has no change-event stream (`set`/`append`) for incremental writers, no snapshot copy-credit economy, and no `replace()` resync when a provider restates full argument text mid-stream.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### AI-UTL-17 — text helpers
- Dimension(s): 19, message semantics
- pi entry: `packages/ai/src/utils/text.ts:6-46` (`contentText` with configurable separator, `getSystemMessageText`, `renderSystemMessageUpdate` framing `Updated/Removed system prompt section "x"`); test `packages/ai/test/text.test.ts`
- Opi entry: `crates/opi-ai/src/message.rs:152-196` (`content_text`, `assistant_content_text`, `output_content_text` — fixed `"\n"` separator), `crates/opi-ai/src/transcript.rs:413-450` (`system_message_text`, `render_system_message_update` identical framing); tests `tests/public_utilities.rs:51-274`
- Comparison: equivalent
- Observable difference: visible-text extraction joins text blocks in order with newline in both; thinking/redacted thinking, tool calls and images never leak. Update framing strings are byte-identical (`'Removed system prompt section "b".'` / `'Updated system prompt section "a":\n\n…'`). pi's `ownedSlice` (`text.ts:48-54`) is a V8 memory optimization with no Rust counterpart — not a behavioral difference. pi's configurable `separator` parameter is not exposed by Opi (only the default `"\n"` paths exist in scope).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-18 — `uuidv7` generator
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/uuid.ts:8-50` (module-global state: rollback floor, 41-bit sequence seeded from bytes[1..5], version/variant bits, hex layout); tests `packages/ai/test/uuid.test.ts`
- Opi entry: `crates/opi-ai/src/identity.rs:44-172` (`UuidV7Generator` with injected `now_ms`, `generate_at` follower path, `UuidV7Error::{InvalidTimestamp,SequenceExhausted}`); tests `tests/public_utilities.rs:779-878`
- Comparison: equivalent
- Observable difference: identical byte layout and ordering behavior (ordinary ids never regress on rollback; explicit follower timestamps preserved exactly without advancing the floor; sequence seeds from bytes[1..5] then increments to 2^41-1 where both fail; boundary `2^48-1` accepted). API shape differs deliberately: pi is a module global reading `Date.now()`; Opi is an instance taking `now_ms` (documented `identity.rs:15-16`), so cross-generator monotonicity is the caller's concern. pi rejects negative/fractional/NaN via `RangeError`; Opi's `u64`/`u64` parameters make those inputs unrepresentable, with the same `InvalidTimestamp` above `2^48-1`.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-19 — `shortHash`
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/hash.ts:2-9`
- Opi entry: `crates/opi-ai/src/identity.rs:23-57`; fixtures `tests/public_utilities.rs:812-834` (values computed from the JS algorithm incl. surrogate pairs and `\u{2248}`)
- Comparison: equivalent
- Observable difference: none for any input — UTF-16 code units (surrogate pairs count twice), `Math.imul` 32-bit wrapping reproduced with `wrapping_mul` on `u32`, sequential final mix, unsigned base36 `h2` then `h1`. Non-cryptographic in both.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-20 — ClientIdentity / User-Agent / attribution
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/identity.ts:11-198` (`UserAgentMode`, `ClientAttribution`, `PI_ATTRIBUTION`, `mergeClientIdentity`, `formatUserAgent`, `getAttributionHeaders` with OpenRouter/NVIDIA-NIM/OpenCode targeting, `getOpenCodeSessionHeaders`, `getIdentityHeaders`, `withClientIdentity` Models wrapper); tests `packages/ai/test/identity.test.ts`
- Opi entry: absent as a framework. Absence search: `grep -rn "withClientIdentity|client_identity|User-Agent|user-agent|attribution" crates/opi-ai/src` → only (a) TLS mTLS `ClientIdentity` (`tls/mod.rs:139`, unrelated), (b) pinned per-provider UAs: Copilot `COPILOT_USER_AGENT` (`auth/oauth/copilot.rs:39-40`), anthropic impersonation UA (`anthropic.rs:1901`), federation UA (`auth/anthropic_federation.rs:300`), (c) Copilot-managed header reservation (`provider.rs:327-345`). Generic per-request headers exist (`provider_headers.rs:37-146`) with no identity model, merge order, or attribution targeting.
- Comparison: missing
- Observable difference: Opi sends no `pi/<version> (<platform>; <runtime>; <arch>)` detailed UA by default, no short/custom UA modes, no OpenRouter `HTTP-Referer`/`X-OpenRouter-Title`/`X-OpenRouter-Categories`, no NVIDIA `X-BILLING-INVOKE-ORIGIN`, no `x-opencode-client`/`x-opencode-session`, and no field-by-field identity merge under model/request headers. Providers can distinguish Opi requests by missing/impersonation-only UA headers.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: Opi's `identity.rs` covers pi's `hash.ts`/`uuid.ts` only (AI-UTL-18/19); the name collision is incidental.

### AI-UTL-21 — tool-argument validation pipeline
- Dimension(s): 19 (validation semantics)
- pi entry: `packages/ai/src/utils/validation.ts:12-82` (WeakMap validator cache, `compile(schema, {formats, coerce:true, unresolvedRefs:"reject"})`, dotted-path error formatting `/edits/0/oldText` → `edits.0.oldText`, error message embeds `Received arguments:\n${JSON.stringify(args)}`); tests `packages/ai/test/validation.test.ts`, `validation-probes.test.ts`
- Opi entry: `crates/opi-ai/src/schema/normalize.rs:38-131` (`validate_tool_arguments`, `validate_tool_call`, `ToolArgumentOutcome::{Valid,Invalid,ArgumentsError}`) over the private TypeBox-dialect engine (`schema/mod.rs:1-31`); tests `tests/schema_public.rs`
- Comparison: partial
- Observable difference: the coercion pipeline is ported faithfully (arguments_error short-circuit before coercion, clone-before-normalize, `normalizeOptionalNulls` incl. `$ref` skip and sub-compile-failure keep, `coerceWithJsonSchema` allOf/anyOf/oneOf/type-union/properties/additionalProperties/items folds, final check against the ORIGINAL schema — with a documented divergence: a coerced root primitive that fails the check reports `Invalid` where upstream returned raw, `normalize.rs:23-26`). Differences: (1) failures are redacted `ToolArgumentFailure {instance_path, keyword, reason}` — raw argument content never enters diagnostics; pi's thrown `Error` embeds the full received arguments JSON. (2) Opi returns a typed outcome/`Err(String)` for unknown tool; pi throws `Tool "X" not found`. (3) pi's TypeBox `Value.Convert` no-op for plain dialect schemas is a documented Opi divergence (`normalize.rs:19-21`). The `unresolvedRefs:"reject"`-at-place behavior (pi `validation.ts:20-22`) maps to Opi `schema/refs.rs` admission, which this batch did not differentially verify.
- Evidence basis: source inspection + test inspection
- Review outcome: unreviewed
- Notes: string-enum validation flows through the same engine on both sides (AI-UTL-22).

### AI-UTL-22 — `StringEnum` schema builder
- Dimension(s): 19
- pi entry: `packages/ai/src/utils/string-enum.ts:14-28` (`{type:"string", enum:[...]}` + optional description/default via truthiness)
- Opi entry: `crates/opi-ai/src/message.rs:646-685` (`string_enum(values, description, default)` → plain `serde_json::Value`); test `tests/public_utilities.rs:455-491`
- Comparison: equivalent
- Observable difference: same portable shape for providers without anyOf/const; empty description/default omitted in both (truthiness mirrored with `filter(!is_empty)`); no TypeBox symbols or JS ABI leak.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-23 — context-overflow and recoverable-length predicates
- Dimension(s): 19 (overflow detection)
- pi entry: `packages/ai/src/utils/overflow.ts:56-198` (25 overflow regexes, 3 non-overflow exclusions, Cerebras bodyless shape provider-gated, silent-overflow `input+cacheRead > window` on `stop`, length-stop `>= 99%` window with zero output; `isRecoverableLength`); tests `packages/ai/test/overflow.test.ts`, `context-overflow.test.ts`
- Opi entry: `crates/opi-ai/src/message.rs:688-832` (`context_overflow_pattern`, `non_context_overflow_pattern`, `cerebras_bodyless_overflow_pattern`, `is_context_overflow`, `is_recoverable_length`); tests `tests/public_utilities.rs:531-726`
- Comparison: equivalent
- Observable difference: pattern sets, exclusion precedence (non-overflow veto first), Cerebras provider gating, and the three-case structure match, including the 99% threshold expressed as `input*100 >= window*99` (`message.rs:802-806`) vs pi's `>= contextWindow * 0.99` — same integer behavior. Two Opi refinements: usage-based cases require `usage.reported` (unknown usage is never zero) and a zero/absent window disables them (`message.rs:792-798`); pi treats only a falsy `contextWindow` param as absent. pi's `getOverflowPatterns()` test accessor has no Opi counterpart (tests assert matches directly).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-24 — diagnostics shapes
- Dimension(s): message semantics
- pi entry: `packages/ai/src/utils/diagnostics.ts:3-49` (`DiagnosticErrorInfo {name?, message, stack?, code?}`, `AssistantMessageDiagnostic {type, timestamp, error?, details?}`, `formatThrownValue`, `extractDiagnosticError`, append helper); Opi counterpart also pinned by `tests/public_utilities.rs:727-776`
- Opi entry: `crates/opi-ai/src/provider.rs:1050-1115` (`DiagnosticErrorInfo {name, message, http_status?, error_name?}` from typed `ProviderError`; no `stack` by design), `crates/opi-ai/src/stream.rs:43-58` (`AssistantDiagnostic {kind, reason, detail?}` — constrained-sampling diagnostics only)
- Comparison: different
- Observable difference: source of diagnostics differs (pi: any thrown `unknown`; Opi: the typed `ProviderError` taxonomy), shape differs (pi carries `timestamp`, free-form `type`, `details` JSON, `stack`, `code`; Opi's message-level `AssistantDiagnostic` is a closed `{kind, reason, detail}` with no timestamp/details, and `createAssistantMessageDiagnostic`/append helper have no counterpart). Persisted diagnostics blocks are therefore not interchangeable.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-25 — event stream / final-result delivery
- Dimension(s): message semantics, streaming
- pi entry: `packages/ai/src/utils/event-stream.ts:26-137` (`EventStream` push/end/async-iterate FIFO with waiting consumers, `result()` promise; `AssistantMessageEventStream` times the final message with a monotonic clock from stream creation unless `durationMs` is set or `timestamp` predates the stream); tests `packages/ai/test/event-stream.test.ts`
- Opi entry: `crates/opi-ai/src/stream.rs:422-520` (`AssistantStreamEvent` taxonomy with snake_case names), `stream.rs:557-675` (`CallFailure`), `stream.rs:698-860` (`ChatStream`: `Stream<Item = Result<AssistantStreamEvent, CallFailure>>`, single absorbed terminal, `result()` driving to completion)
- Comparison: different
- Observable difference: event taxonomy and ordering contract match (`start` before updates, `done` terminal, updates never before start). Delivery differs: pi encodes failures as in-band `{type:"error", error: AssistantMessage}` events on a push-based stream; Opi producers never emit the `Error` terminal (legacy deserialization only, `stream.rs:487-494`) and deliver exactly one `Err(CallFailure)` carrying the canonical partial + typed cause; a stream ending without terminal is a typed protocol failure. Opi `Done.reason` is the full `StopReason` (restricted in pi to `stop|length|toolUse|deferred`, `types.ts:845-848`). Timing differs: pi times from stream creation; Opi's `duration_ms` is adapter-measured from response-stream first poll/activation to terminal and is retained-not-recomputed on forwarding (`message.rs:74-79`). Opi also stamps projection facts onto every event (`stream.rs:484-500`) — opi-only.
- Evidence basis: source inspection + test inspection (`tests/stream_events.rs`)
- Review outcome: self-reviewed

### AI-UTL-26 — whole-call assistant retry
- Dimension(s): 19 (retry semantics)
- pi entry: `packages/ai/src/utils/retry.ts:7-131` (non-retryable limit pattern incl. `subscription_sharing_workplace_individual_usage_limit_exceeded` and z.ai code `1308`; retryable pattern with boundary-protected status codes `(?<![a-z0-9])(?:429|500|…)(?![a-z0-9])` and entries `upstream.?request.?failed`, `upstream_stream_read_error`, `stream was interrupted`, `http/?2 stream failed`, `request to \S+ failed, reason:`, `stream disconnected before completion`), `RetryPolicy`/`retryDelayMs` (`base*2^(attempt-1)` capped 60s), `retryAssistantCall` with callbacks and abort normalization, `isRetryableAssistantError`; tests `packages/ai/test/retry.test.ts`
- Opi entry: `crates/opi-ai/src/retry.rs:866-1290` (`WholeCallRetryPolicy`, `whole_call_retry_delay_ms`, `classify_whole_call_retry`, `non_retryable_limit_pattern` :912, `retryable_transient_pattern` :939, `retry_assistant_call` :1126 with callbacks and cancelled-backoff → `Aborted` normalization dropping the error text); tests `tests/whole_call_retry.rs`
- Comparison: partial
- Observable difference: loop contract matches (disabled policy = one call; aborts terminal and never retried; non-retryable/overflow fails fast; callbacks scheduled→sleep→attempt-start→finished only when a retry was scheduled; delay `base*2^(attempt-1)` saturating, default cap 60 s). Pattern-list drift: Opi's non-retryable set omits `subscription_sharing_workplace_individual_usage_limit_exceeded` and the z.ai `1308` code pattern (:912-937); Opi's transient set omits `upstream_request_failed`, `upstream_stream_read_error`, `stream was interrupted`, `http/2 stream failed`, `request to \S+ failed, reason:`, `stream disconnected before completion` (:939-1015) and matches bare `429/500/…` without the no-alphanumeric-boundary lookarounds (pi's #11057 fix), so e.g. an id containing "500" can misclassify. Opi additionally vetoes context-overflow errors inside the classifier (:1030-1035) where pi's doc leaves overflow handling to the caller before applying retry (`overflow.ts` doc; `retry.ts:265-269`). Opi also has a separate acquisition-retry core (`retry.rs:179-860`) with no pi `utils/retry.ts` counterpart (it mirrors SDK-internal retries, not this file).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-27 — credential mutation queue
- Dimension(s): 19 (adjacent store contract)
- pi entry: nearest concept is `packages/ai/src/auth/types.ts:52` ("`modify` is the only write path, so every mutation is a…") plus `models.ts:791-824` using `credentials.modify`; no queue/generation/barrier/receipt machinery anywhere (absence search: `grep -rn "mutation|barrier|Barrier" .pi/pi-1.5.0/packages/ai/src` → only those auth/models lines)
- Opi entry: `crates/opi-ai/src/mutation.rs:1-704` (`MutationQueue` store-owned serialized mutations: generations, per-key FIFO, detached execution, `MutationReceipt`/`MutationOutcome` incl. `Unknown` on worker panic, `MutationBarrier` cutoff + fenced late writes, queue-backed `InMemoryCredentialStore`)
- Comparison: opi-only
- Observable difference: Opi gives shared credential stores serialized read-modify-write with ACKs and store-wide barriers; pi's `modify` is a single callback seam without admission numbering, receipts, or barrier reporting. Dropping an Opi waiter never cancels a started mutation.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Layer H: module is explicitly `# Unstable — unstable 0.x extension substrate` (`mutation.rs:21-23`), implemented.

### AI-UTL-28 — replay projection vs `transformMessages`
- Dimension(s): 19, message semantics
- pi entry: `packages/ai/src/api/transform-messages.ts:91-256` (`transformMessages`: image downgrade per run, cross-model thinking dropped / same-model keeps signed+redacted and drops blank unsigned, cross-model `textSignature`/`thoughtSignature` stripped, caller-supplied `normalizeToolCallId` with shared id map, error/aborted assistant turns skipped with linked results dropped via pending-call accounting, synthetic `"No result provided"` error results, system messages held past open batches); `splitErrorToolResultImages` :31-45; tests `transform-messages-cross-model-thinking.test.ts`, `transform-messages-copilot-openai-to-anthropic.test.ts`
- Opi entry: `crates/opi-ai/src/replay.rs:627-781` (`project_replay`) + anthropic/bedrock error-image split (`anthropic.rs:2648-2669`, `bedrock/mod.rs:1788,2277`); tests `src/replay.rs:786-1500`, `tests/replay_contract{.rs,/object_wire.rs}`
- Comparison: partial
- Observable difference: shared semantics: same placeholders `(image omitted: model does not support images)` / `(tool image omitted…)` per image RUN, error/aborted turns omitted with their linked results, blank unsigned thinking dropped, signed/redacted kept same-identity, system messages batch-transparent. Divergences: (1) association errors — Opi raises typed `DuplicateCallInBatch`/`DuplicateResult`/`OrphanResult`/`ResultNameMismatch` (`replay.rs:155-193`); pi silently keeps late/duplicate results and only synthesizes for unanswered calls. (2) synthetic missing-result text: pi `"No result provided"`; Opi a longer "Result unavailable … execution effect is unknown" text plus a user-role failure context for omitted batches (`replay.rs:331-346`). (3) identity: pi same-model = provider+api+model on the message; Opi = stored `wire` + provider + model, fail-closed for legacy records without `wire` (`replay.rs:198-203`; pi has no stored-wire concept). (4) malformed proposals (`arguments_error`) get wire-dependent handling with an explicit `[opi:tool-call-not-executed:v1]` failure context (`replay.rs:783`); pi has no malformed-proposal concept (arguments are always objects). (5) wire-ID legalization is built-in per wire with Anthropic/Bedrock charset+64 limits (`replay.rs:213-268`); pi delegates to a caller callback. (6) Opi records `ReplayLoss` facts and supports `Strict` rejection; pi has neither. (7) pi strips cross-model `textSignature`/`thoughtSignature` in this pass; Opi's replay keeps Text blocks intact and defers signature echo rules to adapters (per `message.rs:56-60` docs).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed
- Notes: Opi pins pi 1.3.0's transform-messages (`replay.rs:11-13`); pi 1.5.0 added `splitErrorToolResultImages` (#10794), which Opi has in the two adapters.

### AI-UTL-29 — streaming UTF-8 chunk decoder
- Dimension(s): 19 (chunk boundary semantics)
- pi entry: platform `TextDecoder` with `{stream:true}` per SSE reader (`packages/ai/src/api/anthropic-messages.ts:482`, `mistral-conversations.ts:456`, `openai-codex-responses.ts:820`, `pi-messages.ts:295`); WHATWG replacement semantics
- Opi entry: `crates/opi-ai/src/text_decode.rs:11-84` (`StreamTextDecoder::push/finish`: carry incomplete sequences, one U+FFFD per maximal invalid subsequence, flush tail at EOF); boundary-split tests :88-190
- Comparison: equivalent
- Observable difference: none observed for legal UTF-8 under every byte-boundary split and for invalid sequences (one replacement per maximal invalid subsequence, matching WHATWG non-fatal decoding). Both are UTF-8; neither side does UTF-16 chunk decoding in scope (pi's `sanitize-unicode.ts`/`unicode-surrogate.test.ts` handle other concerns, not inspected here).
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

### AI-UTL-30 — context-occupancy estimation (adjacent to overflow trimming)
- Dimension(s): 19 (budgeting, adjacent)
- pi entry: `packages/ai/src/utils/estimate.ts:1-91` (3.5 chars/token ceil, 4800 chars/image, per-message rounding, last applicable positive usage anchor with prefix-timestamp invalidation, `calculateContextTokens = totalTokens || buckets`); test `packages/ai/test/context-estimate.test.ts`
- Opi entry: `crates/opi-ai/src/context.rs:1-200` (same 2/7 rate, 4800/image, same per-message rounding incl. system body+sections once plus each tool list, usage anchor with prefix invalidation and error/aborted exclusion); tests `tests/context_budget.rs`
- Comparison: partial
- Observable difference: anchor selection differs where usage is unknown or totals disagree: Opi's `anchor_tokens` requires a positive reported total else a reported positive bucket sum, and never converts unknown usage to zeros (`context.rs:91-107`); pi uses `totalTokens || input+output+cacheRead+cacheWrite` with no reported-ness concept (pi `Usage` numbers are always present). Assistant tool-call estimate: pi stringifies the parsed `arguments` object; Opi measures the raw argument text (pre-repair model text can differ from canonical JSON). Opi counts `system`/`tools` parameters only when no anchor exists because its entry point takes them separately (`context.rs:132-146`); pi's `TranscriptContext` always embeds them.
- Evidence basis: source inspection + test inspection
- Review outcome: self-reviewed

## Layer H — active design material encountered in scope

- pi `transcript.ts:180-186` `hasToolRedefinitions` — authority wording: `@deprecated` "Kept for API compatibility and will be removed in a future release" (normative deprecation); maturity: implemented + tested.
- pi `text.ts:28-37` `renderSystemMessageUpdate` — "This framing is request-time only and may change between versions" (explicit non-durable contract); implemented.
- pi `streaming-json.ts:1-35` module comment — normative cost/behavior contract for the incremental parser (linear-time append, snapshot credit economy, documented divergences from `parseStreamingJson`); implemented + tested. No Opi counterpart (AI-UTL-16).
- pi `overflow.ts:118-132` "Custom Providers" section — normative guidance: add a regex to `OVERFLOW_PATTERNS` for custom-provider overflow; implemented documentation.
- pi `identity.ts:26-28` `PI_ATTRIBUTION` — "pi's attribution, used by pi-agent-core, pi-durable, and the coding agent"; implemented. Opi has no counterpart (AI-UTL-20).
- pi `event-stream.ts:88-96` duration timing contract — "A stream that forwards a response which started elsewhere … leaves it untimed"; implemented.
- pi `retry.ts:127-133` `RetryPolicy` placement rationale — "kept here so the classifier and the policy-driven retry loop live together and stay reusable"; implemented.
- pi `transcript-tool-changes.test.ts:28-33,208-210` — active experimental provider behaviors: Anthropic `inline-tools-2026-09-15` beta, OpenAI `additional_tools`/`tool_search`, Kimi K3 redeclaration behavior "verified October 2026"; all implemented and payload-pinned.
- Opi `mutation.rs:21-23` — `# Unstable` "unstable 0.x extension substrate. Breaking changes may occur between minor versions"; implemented.
- Opi `crypto/mod.rs:1-19` — private substrate claim ("no local FFI, crypto fork … not re-exported"); implemented, out of this batch's pi scope.
- Opi `transcript.rs:24-27`, `replay.rs:11-13`, `message.rs`/`provider.rs` pinned-source comments — all pin `.repo/pi-1.3.0`, one minor behind the 1.5.0 snapshot being measured; the pins are the documented reproduction baseline, and several `different` verdicts above trace to that one-version lag rather than to local invention.

## Coverage honesty

- Not inspected: pi `utils/{abort,abort-signals,error-body,estimate→(read),headers,model-operations,models-error,node-http-proxy,oauth-page,openrouter-cost,provider-env,provider-retry,sanitize-unicode,sleep,version}.ts` (other batches or out of this batch's file list; `estimate.ts` was read only because measure item 4 mentions trimming); pi `api/*` adapters beyond the SSE decoder lines and `transform-messages.ts`; pi non-`ai` packages; Opi adapters other than the cited argument-accumulation, UA and error-image-split lines; Opi `schema/` internals beyond `mod.rs`/`normalize.rs` (the engine's `refs.rs`/`eval.rs` behavior was not differentially verified against `unresolvedRefs:"reject"`); Opi `catalog/`, `ws/`, `tls/`, `auth/` beyond cited lines; no tests were executed (read-only inspection per `00-inputs.md`).
- `partial-json` (npm) completion heuristics were compared only at the contract level (both cascade positions and never-fail), not differentially fuzzed against Opi's `complete_partial_json`.
- pi session-file/versioning: confirmed absent inside `packages/ai` scope by reading `types.ts`/`transcript.ts`; pi's coding-agent session persistence was not opened (out of scope).
