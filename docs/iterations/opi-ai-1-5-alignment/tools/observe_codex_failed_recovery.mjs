import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { zstdDecompressSync } from "node:zlib";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_failed_recovery.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/src", path)).href);
const [{ stream, closeOpenAICodexWebSocketSessions, resetOpenAICodexWebSocketDebugStats },
  { normalizeContext }] = await Promise.all([
  load("api/openai-codex-responses.ts"),
  load("utils/transcript.ts"),
]);
const { frames, cases } = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-failed-recovery.json", import.meta.url,
), "utf8"));
const context = (...messages) => normalizeContext({
  systemPrompt: "system prompt",
  messages: messages.map((content) => ({ role: "user", content, timestamp: 0 })),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const model = {
  id: "gpt-5.4", name: "GPT-5.4", api: "openai-codex-responses", provider: "openai-codex",
  baseUrl: "https://codex.fixture.invalid", reasoning: true, input: ["text", "image"],
  contextWindow: 272000, maxTokens: 128000,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
};
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");
const eventsFor = (groups) => groups.flatMap((group) => frames[group]);

async function observe(row) {
  const transport = row.warm ? "websocket-cached" : row.ws.length ? "auto" : "sse";
  const sent = [];
  const posts = [];
  const observed = [];
  const publicEvents = [];
  const rawEvents = [];
  let connections = 0;
  globalThis.WebSocket = class extends EventTarget {
    constructor(url) {
      super();
      assert.equal(url, "wss://codex.fixture.invalid/codex/responses");
      this.connection = ++connections;
      queueMicrotask(() => this.dispatchEvent(new Event("open")));
    }

    send(data) {
      const { type, ...body } = JSON.parse(data);
      assert.equal(type, "response.create");
      const index = sent.length - Number(Boolean(row.warm));
      sent.push({ body, connection: this.connection });
      const events = index < 0 ? frames.warmup : eventsFor(row.ws[index]);
      queueMicrotask(() => {
        for (const event of events) {
          this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify(event) }));
        }
      });
    }

    close() {}
  };

  const call = (input) => stream(model, input, {
    apiKey: `fixture.${account}.fixture`, sessionId: "session-fixed", cacheRetention: "short",
    transport, temperature: 0.2, reasoningEffort: "high", maxRetries: 0,
    onProviderStreamEvent(event) {
      rawEvents.push(event.type);
    },
    async fetch(url, init) {
      assert.equal(String(url), "https://codex.fixture.invalid/codex/responses");
      assert.equal(init.method, "POST");
      assert(row.sse, "unexpected SSE POST");
      const body = typeof init.body === "string" ? init.body : zstdDecompressSync(init.body).toString();
      posts.push(JSON.parse(body));
      const text = eventsFor(row.sse).map((event) => `data: ${JSON.stringify(event)}\n\n`).join("");
      return new Response(text, { status: 200, headers: { "content-type": "text/event-stream" } });
    },
  });

  try {
    if (row.warm) {
      const warm = await call(context("hello")).result();
      assert.equal(warm.responseId, "resp_warm");
      assert.equal(warm.stopReason, "stop");
      rawEvents.length = 0;
    }
    const source = call(row.warm ? context("hello", "second turn") : context("hello"));
    for await (const event of source) {
      publicEvents.push(event.type);
      if (event.type === "text_delta") observed.push(event.delta);
    }
    const message = await source.result();
    const text = message.content.filter((part) => part.type === "text").map((part) => part.text).join("");
    assert.equal(message.stopReason, row.piStop ?? row.stop, row.name);
    assert.equal(text, row.piText ?? row.text, row.name);
    assert.equal(observed.join(""), text, row.name);
    assert.equal(publicEvents.filter((type) => type === "done" || type === "error").length, 1, row.name);
    const attempts = sent.slice(Number(Boolean(row.warm)));
    assert.equal(attempts.length, row.ws.length, row.name);
    assert.equal(posts.length, Number(Boolean(row.sse)), row.name);
    if (row.warm) {
      assert.equal(attempts[0].connection, sent[0].connection);
      assert.equal(attempts[0].body.previous_response_id, "resp_warm");
      assert.equal(attempts[0].body.input.length, 1);
      assert.notEqual(attempts[1].connection, attempts[0].connection);
      assert.equal(attempts[1].body.previous_response_id, undefined);
      assert.equal(attempts[1].body.input.length, 2);
    }
    if (attempts.length && posts.length) assert.deepEqual(posts[0], attempts.at(-1).body);
    return {
      name: row.name, ws: attempts.length, sse: posts.length,
      inputs: attempts.map(({ body }) => ({
        previous: body.previous_response_id ?? null, items: body.input.length,
      })),
      stop: message.stopReason, text, deliveredText: observed.join(""),
      responseId: message.responseId ?? null, totalTokens: message.usage.totalTokens,
      rawStop: message.rawStopReason ?? null, publicEvents, rawEvents,
    };
  } finally {
    closeOpenAICodexWebSocketSessions("session-fixed");
    resetOpenAICodexWebSocketDebugStats("session-fixed");
  }
}

const rows = [];
for (const row of cases) rows.push(await observe(row));
console.log(JSON.stringify(rows));
