# Batch AI-STR/AI-AUTH — streams, transport, auth, typed ops (Dims 2,4,16)

pi side: `packages/ai/src/api/` (`anthropic-messages*.ts`,
`openai-completions*.ts`, `openai-responses*.ts`, `openai-codex-responses*.ts`,
`google-generative-ai*.ts`, `google-vertex*.ts`, `google-shared.ts`,
`bedrock-converse-stream*.ts`, `mistral-conversations*.ts`, `pi-messages*.ts`,
`classifier-shared.ts`, `llama-cpp-classify*.ts`, `openrouter-images*.ts`,
`constrained-sampling.ts`, `transform-messages.ts`, `simple-options.ts`,
`openai-prompt-cache.ts`, `github-copilot-headers.ts`, `azure-openai-config.ts`,
`system-one-shared.ts`, `typesafe-system-one*.ts`, `cloudflare*.ts`),
`auth/` (`context.ts`, `credential-store.ts`, `helpers.ts`, `types.ts`),
`compat/extension-oauth-types.ts`, `session-resources.ts`,
`utils/{retry.ts, event-stream.ts, diagnostics.ts}`, `types.ts`.

Opi side: `crates/opi-ai/src/{api.rs, api/, api_mapped.rs, stream.rs,
callbacks.rs, cancel.rs, retry.rs, deferred.rs, http.rs, ws.rs, ws/, tls/,
auth.rs, auth/, credential.rs, context.rs, time.rs, classifier.rs, images.rs,
constrained_sampling.rs}`.

Measure:
1. Streaming event contract: every pi StreamEvent kind/payload/order (start,
   chunk types, reasoning, tool-call deltas, partials, usage) vs opi event
   enum; name the diffs.
2. Request lifecycle: retry (conditions/backoff/attempts), fallback, timeout
   phases, cancellation (AbortSignal vs opi cancel.rs), deferred fetch.
3. Auth: credential store format/location, env precedence per provider,
   OAuth login/refresh/logout/callback flows per provider, side-effect-free
   status checks, ADC/federation analogs.
4. Adapter deep dives — anthropic, openai-responses, openai-completions,
   google: URL, headers, body fields, thinking config, tool_choice, image/file
   parts, stream parsing, error taxonomy, usage extraction.
5. Typed ops: image generation (`openrouter-images`) and classifiers
   (`classifier-shared`, `llama-cpp-classify`) vs `images.rs`, `classifier.rs`.
6. `session-resources.ts` ownership/cleanup vs opi equivalent.

Output `evidence/ai-streams-auth.md`, prefixes `AI-STR`, `AI-AUTH`.
Tallies + coverage honesty; exact absence-search commands.
