import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { zstdDecompressSync } from "node:zlib";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_connection_limit.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/src", path)).href);
const [{ stream, closeOpenAICodexWebSocketSessions, resetOpenAICodexWebSocketDebugStats },
  { normalizeContext }] = await Promise.all([
  fromSnapshot("api/openai-codex-responses.ts"),
  fromSnapshot("utils/transcript.ts"),
]);
const fixture = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-connection-limit.json", import.meta.url,
), "utf8"));
const sse = fixture.completion.map((event) => `data: ${JSON.stringify(event)}\n\n`).join("");
const model = {
  id: "gpt-5.4", name: "GPT-5.4", api: "openai-codex-responses", provider: "openai-codex",
  baseUrl: "https://codex.fixture.invalid", reasoning: true, input: ["text", "image"],
  contextWindow: 272000, maxTokens: 128000,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
};
const context = (...messages) => normalizeContext({
  systemPrompt: "system prompt",
  messages: messages.map((content) => ({ role: "user", content, timestamp: 0 })),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");

async function observe(transport, { progress = false, warm = false, sseFailureRetries } = {}) {
  let connections = 0;
  let closed = 0;
  let sent = 0;
  const requests = [];
  const callbacks = [];
  globalThis.WebSocket = class extends EventTarget {
    constructor(url, { headers }) {
      super();
      assert.equal(url, "wss://codex.fixture.invalid/codex/responses");
      assert.equal(new Headers(headers).get("chatgpt-account-id"), "acct-1");
      connections++;
      queueMicrotask(() => this.dispatchEvent(new Event("open")));
    }

    send(data) {
      const { type, ...body } = JSON.parse(data);
      assert.equal(type, "response.create");
      requests.push({ transport: "websocket", body });
      const warmup = ++sent === 1 && warm;
      const frames = warmup ? [fixture.warmup] : progress ? [fixture.progress, fixture.error] : [fixture.error];
      queueMicrotask(() => {
        for (const frame of frames) {
          this.dispatchEvent(new MessageEvent("message", { data: JSON.stringify(frame) }));
        }
      });
    }

    close() {
      closed++;
    }
  };

  async function call(inputContext) {
    const resultStream = stream(model, inputContext, {
      apiKey: `fixture.${account}.fixture`,
      transport, sessionId: "session-fixed", cacheRetention: "short", maxRetries: sseFailureRetries ?? 0,
      temperature: 0.2, reasoningEffort: "high",
      onPayload() {
        callbacks.push("payload");
      },
      onProviderStreamEvent(event) {
        callbacks.push(`raw:${event.type}`);
      },
      onResponse(response) {
        callbacks.push(`response:${response.status}`);
      },
      async fetch(input, init) {
        assert.equal(String(input), "https://codex.fixture.invalid/codex/responses");
        assert.equal(init.method, "POST");
        assert.equal(new Headers(init.headers).get("chatgpt-account-id"), "acct-1");
        assert.equal(closed, 2, "both rejected connections released before POST");
        const body = typeof init.body === "string" ? init.body : zstdDecompressSync(init.body).toString();
        requests.push({ transport: "sse", body: JSON.parse(body) });
        if (sseFailureRetries !== undefined) throw new TypeError("fixture closed the POST before headers");
        return new Response(sse, { status: 200, headers: { "content-type": "text/event-stream" } });
      },
    });
    const events = [];
    for await (const event of resultStream) events.push(event.type);
    const message = await resultStream.result();
    const text = message.content.filter((part) => part.type === "text").map((part) => part.text).join("");
    assert.equal(events.filter((type) => type === "done" || type === "error").length, 1);
    return {
      stop: message.stopReason, text, responseId: message.responseId ?? null,
      totalTokens: message.usage.totalTokens, events,
      diagnostic: message.diagnostics?.[0]?.type ?? null,
    };
  }

  try {
    if (warm) {
      const warmup = await call(context("hello"));
      assert.equal(warmup.responseId, "resp_warm");
      assert.equal(warmup.stop, "stop");
      assert.equal(connections, 1);
      assert.equal(closed, 0);
      requests.length = 0;
      callbacks.length = 0;
    }
    const inputContext = warm ? context("hello", "second turn") : context("hello");
    const first = await call(inputContext);
    const firstSends = requests.map((request) => request.transport);
    const firstCallbacks = callbacks.slice();
    const body = requests.at(-1).body;
    if (progress) {
      assert.equal(connections, 1);
      assert.equal(closed, 1);
      assert.deepEqual(firstSends, ["websocket"]);
      assert.equal(first.stop, "error");
      assert.deepEqual(first.events, ["start", "error"]);
      return { transport, progress, warm, connections, closed, firstSends, firstCallbacks, body, first };
    }
    if (sseFailureRetries !== undefined) {
      assert.deepEqual(firstSends, ["websocket", "websocket", ...Array(1 + sseFailureRetries).fill("sse")]);
      assert.equal(first.stop, "error");
      assert.equal(first.text, "");
      assert.deepEqual(first.events, ["error"]);
      return { transport, sseFailureRetries, connections, closed, firstSends, firstCallbacks, body, first };
    }
    if (warm) {
      const delta = requests[0].body;
      assert.equal(delta.previous_response_id, "resp_warm");
      assert.equal(delta.input.length, 1);
      assert.equal(body.previous_response_id, undefined);
      assert.equal(body.input.length, 2);
    }

    assert.deepEqual(firstSends, ["websocket", "websocket", "sse"]);
    assert.equal(first.stop, "stop");
    assert.equal(first.text, "Hello from SSE");
    assert.equal(first.responseId, "resp_sse");
    assert.equal(first.totalTokens, 8);
    assert.deepEqual(firstCallbacks, [
      "payload", "raw:error", "raw:error", "response:200",
      ...fixture.completion.map((event) => `raw:${event.type}`),
    ]);
    assert.equal(first.diagnostic, "provider_transport_failure");
    const second = await call(inputContext);
    assert.equal(connections, 2);
    assert.equal(closed, 2);
    const secondSends = requests.slice(firstSends.length).map((request) => request.transport);
    assert.deepEqual(secondSends, ["sse"]);
    assert.deepEqual(second, { ...first, diagnostic: null });
    for (const request of requests.slice(warm ? 1 : 0)) assert.deepEqual(request.body, body);
    return {
      transport, progress, warm, connections, closed, firstSends, firstCallbacks, body, first, secondSends, second,
    };
  } finally {
    closeOpenAICodexWebSocketSessions("session-fixed");
    resetOpenAICodexWebSocketDebugStats("session-fixed");
  }
}

const rows = [];
for (const transport of ["auto", "websocket", "websocket-cached"]) {
  rows.push(await observe(transport));
}
rows.push(await observe("websocket-cached", { warm: true }));
rows.push(await observe("auto", { progress: true }));
rows.push(await observe("auto", { sseFailureRetries: 0 }));
rows.push(await observe("auto", { sseFailureRetries: 1 }));
console.log(JSON.stringify(rows));
