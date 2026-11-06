# Batch AI-UTL — transcripts, data contracts (Dim 19 + message semantics)

pi side: `packages/ai/src/utils/{json-parse.ts, streaming-json.ts, text.ts,
transcript.ts, validation.ts, overflow.ts, identity.ts, uuid.ts,
string-enum.ts, diagnostics.ts, event-stream.ts, retry.ts}`, message and
system-update types in `packages/ai/src/types.ts`, transcript fixtures under
`packages/ai/test/` that pin semantics.

Opi side: `crates/opi-ai/src/{transcript.rs, message.rs, schema/, identity.rs,
text_decode.rs, crypto/, mutation.rs, replay.rs}` and their tests.

Measure:
1. Transcript format: session JSON structure, message envelope fields, system
   message + system-update (render/update) semantics, normalization rules,
   version/migration if any.
2. JSON parsing: `json-parse` error tolerance/recovery, `streaming-json`
   incremental parser states; opi equivalents incl. `text_decode` chunk
   boundary semantics (UTF-8 vs UTF-16).
3. `identity.ts`/`uuidv7` vs `identity.rs`: format, ordering, fixtures.
4. `validation.ts` + `overflow.ts`: context overflow detection/trimming,
   string-enum semantics; opi counterpart.
5. `mutation.rs`/`replay.rs`: pi counterpart (search) or mark opi-only with
   behavior description.

Output `evidence/ai-utils.md`, prefix `AI-UTL`. Tallies + coverage honesty;
exact absence-search commands.
