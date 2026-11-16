import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { zstdDecompressSync } from "node:zlib";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_terminal.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/src", path)).href);
const [{ stream }, { normalizeContext }, { isRetryableAssistantError }] = await Promise.all([
  load("api/openai-codex-responses.ts"),
  load("utils/transcript.ts"),
  load("utils/retry.ts"),
]);
const fixture = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-terminal.json", import.meta.url,
), "utf8"));
const context = normalizeContext({
  systemPrompt: "system prompt",
  messages: [{ role: "user", content: "hello", timestamp: 0 }],
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

async function observe(row, transport) {
  const content = row.content ?? "text";
  const terminal = { type: row.type, response: { ...fixture.response, ...row.fields } };
  const frames = [...fixture.content[content], terminal];
  const expectedRaw = frames.map(({ type }) => type);
  if (row.late) frames.push(fixture.late);
  const raw = [];
  const publicEvents = [];
  const deliveredText = [];
  const sends = [];
  let ws = 0;
  let sse = 0;
  globalThis.WebSocket = class extends EventTarget {
    constructor(url) {
      super();
      assert.equal(url, "wss://codex.fixture.invalid/codex/responses");
      queueMicrotask(() => this.dispatchEvent(new Event("open")));
    }

    send(data) {
      ws++;
      const { type, ...body } = JSON.parse(data);
      assert.equal(type, "response.create");
      sends.push(body);
      queueMicrotask(() => {
        for (const event of frames) {
          this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify(event) }));
        }
        queueMicrotask(() => {
          this.dispatchEvent(Object.assign(new Event("close"), { code: 1000, wasClean: true }));
        });
      });
    }

    close() {}
  };
  const source = stream(model, context, {
    apiKey: `fixture.${account}.fixture`, transport,
    temperature: 0.2, reasoningEffort: "high", maxRetries: 0,
    onProviderStreamEvent(event) { raw.push(event); },
    async fetch(url, init) {
      sse++;
      assert.equal(String(url), "https://codex.fixture.invalid/codex/responses");
      assert.equal(init.method, "POST");
      const body = typeof init.body === "string" ? init.body : zstdDecompressSync(init.body).toString();
      sends.push(JSON.parse(body));
      const text = frames.map((event) => `data: ${JSON.stringify(event)}\n\n`).join("");
      return new Response(text, { status: 200, headers: { "content-type": "text/event-stream" } });
    },
  });
  for await (const event of source) {
    publicEvents.push(event.type);
    if (event.type === "text_delta") deliveredText.push(event.delta);
  }
  const message = await source.result();
  const stop = message.stopReason === "toolUse" ? "tool_use" : message.stopReason;
  const text = message.content.filter((part) => part.type === "text").map((part) => part.text).join("");
  const tools = message.content.filter((part) => part.type === "toolCall")
    .map(({ id, name, arguments: args }) => ({ id, name, args }));
  assert.equal(stop, row.stop, row.name);
  assert.equal(message.rawStopReason, row.rawStop, row.name);
  assert.equal(message.endTurn, row.endTurn, row.name);
  assert.equal(text, content === "text" ? "partial" : "", row.name);
  assert.equal(deliveredText.join(""), text, row.name);
  assert.equal(message.responseId, fixture.response.id, row.name);
  assert.equal(message.usage.totalTokens, fixture.response.usage.total_tokens, row.name);
  assert.deepEqual(raw.map(({ type }) => type), expectedRaw, row.name);
  assert.deepEqual(publicEvents.filter((type) => type === "done" || type === "error"),
    [row.stop === "error" ? "error" : "done"], row.name);
  assert.equal(sse, Number(transport === "sse"), row.name);
  assert.equal(ws, Number(transport === "websocket"), row.name);
  assert.equal(sends.length, 1, row.name);
  return {
    name: row.name, transport, ws, sse, stop, text, tools,
    deliveredText: deliveredText.join(""), responseId: message.responseId,
    usage: {
      input: message.usage.input, output: message.usage.output, cacheRead: message.usage.cacheRead,
      reasoning: message.usage.reasoning, total: message.usage.totalTokens,
    },
    rawStop: message.rawStopReason ?? null, endTurn: message.endTurn ?? null,
    rawEndTurn: raw.at(-1)?.response?.end_turn ?? null,
    publicEvents, rawEvents: raw.map(({ type }) => type),
    diagnostic: message.errorMessage ?? null,
    retryable: isRetryableAssistantError(message),
  };
}

const observations = [];
for (const row of fixture.cases) {
  for (const transport of ["sse", "websocket"]) observations.push(await observe(row, transport));
}
console.log(JSON.stringify(observations));
