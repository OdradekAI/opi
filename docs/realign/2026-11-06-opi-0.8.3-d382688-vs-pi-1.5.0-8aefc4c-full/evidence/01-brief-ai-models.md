# Batch AI-MOD — model catalog & provider registry (Dims 1,3,16)

pi side (root `.pi/pi-1.5.0`): `packages/ai/src/models.ts`, `models-store.ts`,
`providers/` (registry `all.ts`; ~37 provider modules incl. `*.models.ts` data,
`compat-schema.ts`, `model-schema.ts`, `schema-builders.ts`, `data-json.d.ts`,
`cloudflare-auth.ts`, `cloudflare-stream.ts`, `opencode-*`, `radius*`),
`catalog/` (`generate.ts`, `build.ts`, `define.ts`, `families/`, `sources/`,
`reasoning.ts`), `api/lazy.ts`, `scripts/{model-data.ts,hydrate-model-catalog.ts,check-model-data.ts}`.

Opi side (root repo): `crates/opi-ai/src/{models.rs, model_info.rs, provider.rs,
provider_collection.rs, registry.rs, provider_env.rs, provider_headers.rs,
providers/, catalog/, endpoint.rs}` plus per-family modules `anthropic.rs,
openai_chat.rs, openai_responses.rs, openai_responses_shared.rs,
openai_codex_responses.rs(+dir), azure_openai.rs, bedrock/, gemini.rs,
vertex.rs, mistral.rs, openrouter.rs, github_copilot.rs, pi_messages.rs,
codex_resources.rs`.

Measure:
1. Provider registry: enumerate pi provider ids/names vs opi; rough model
   counts per provider (`*.models.ts` vs opi catalog source); providers
   present only on one side.
2. ModelInfo/Model record fields: ids, context lengths, pricing fields+units,
   capability flags, thinking levels, defaults and exact semantics.
3. Catalog pipeline: generation/pinning/hydration (`catalog/*`, `scripts/`),
   models-store cache/ETag/offline/refresh/stale-generation rejection vs opi
   catalog module (static vs dynamic, format, refresh behavior).
4. Custom/user model overlays in `models.ts` (paths, precedence).
5. ModelType coverage: chat/image/classifier/embedding records both sides.

Priorities: registry enumeration + models-store semantics; deep-dive
anthropic + openai as representatives. Output `evidence/ai-models.md`,
prefix `AI-MOD`. Verdict tallies + coverage honesty; exact absence-search
commands for anything claimed missing.
