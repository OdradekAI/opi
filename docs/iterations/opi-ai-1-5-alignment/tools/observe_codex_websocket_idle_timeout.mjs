import assert from "node:assert/strict";
import { once } from "node:events";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { pathToFileURL } from "node:url";
import { zstdDecompressSync } from "node:zlib";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_websocket_idle_timeout.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ stream, closeOpenAICodexWebSocketSessions }, { normalizeContext },
  { isRetryableAssistantError }, { WebSocketServer }] = await Promise.all([
  load("packages/ai/src/api/openai-codex-responses.ts"),
  load("packages/ai/src/utils/transcript.ts"),
  load("packages/ai/src/utils/retry.ts"),
  load("node_modules/ws/wrapper.mjs"),
]);
const fixture = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-websocket-idle-timeout.json", import.meta.url,
), "utf8"));
const sse = fixture.frames.sse.map((event) => `data: ${JSON.stringify(event)}\n\n`).join("");
const context = (...messages) => normalizeContext({
  systemPrompt: "system prompt",
  messages: messages.map((content) => ({ role: "user", content, timestamp: 0 })),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");

async function observe(row, transport) {
  const sessionId = `idle-timeout-${row.name}-${transport}`;
  const creates = [];
  const posts = [];
  let connections = 0;
  let sawPong = false;
  let close = null;
  const expiredClose = Promise.withResolvers();
  const server = createServer(async (request, response) => {
    assert.equal(request.method, "POST");
    assert.equal(request.url, "/codex/responses");
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const data = Buffer.concat(chunks);
    const body = JSON.parse(request.headers["content-encoding"] === "zstd" ? zstdDecompressSync(data) : data);
    posts.push({ items: body.input.length, previous: body.previous_response_id ?? null });
    response.writeHead(200, { "content-type": "text/event-stream", connection: "close" }).end(sse);
  });
  const websocket = new WebSocketServer({ noServer: true });
  server.on("upgrade", (request, socket, head) => {
    assert.equal(request.url, "/codex/responses");
    websocket.handleUpgrade(request, socket, head, (ws) => {
      const connection = ++connections;
      let timer;
      let sentLate = false;
      const send = (group) => {
        for (const event of fixture.frames[group]) ws.send(JSON.stringify(event));
      };
      ws.on("pong", (data) => {
        assert.equal(data.toString(), "heartbeat");
        sawPong = true;
      });
      ws.on("close", (code, reason) => {
        clearInterval(timer);
        if (connection === 1 && !sentLate) {
          close = { code, reason: reason.toString() };
          expiredClose.resolve();
        }
      });
      ws.on("message", (data) => {
        const body = JSON.parse(data);
        assert.equal(body.type, "response.create");
        creates.push({ connection, items: body.input.length, previous: body.previous_response_id ?? null });
        if (creates.length !== 1) {
          send("next");
          return;
        }
        send(row.prefix);
        let ticks = 0;
        timer = setInterval(() => {
          if (ws.readyState !== 1) return;
          if (row.traffic === "ping") ws.ping("heartbeat");
          if (row.traffic === "pong") ws.pong("heartbeat");
          if (row.traffic === "progress") send("progress");
          if (++ticks === fixture.ticks) {
            clearInterval(timer);
            sentLate = true;
            send("late");
          }
        }, fixture.tickMs);
      });
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const model = {
    id: "gpt-5.4", name: "GPT-5.4", api: "openai-codex-responses", provider: "openai-codex",
    baseUrl: `http://127.0.0.1:${server.address().port}`, reasoning: true, input: ["text", "image"],
    contextWindow: 272000, maxTokens: 128000,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
  async function call(input, timeoutMs, holdCallback) {
    const raw = [];
    const terminal = [];
    const source = stream(model, input, {
      apiKey: `fixture.${account}.fixture`, transport, sessionId, timeoutMs,
      cacheRetention: "short", reasoningEffort: "high", temperature: 0.2, maxRetries: 0,
      signal: AbortSignal.timeout(4_000),
      async onProviderStreamEvent(event) {
        raw.push(event.type);
        if (holdCallback && event.type === "response.in_progress") await delay(fixture.callbackHoldMs);
      },
    });
    for await (const event of source) {
      if (event.type === "done" || event.type === "error") terminal.push(event.type);
    }
    const message = await source.result();
    return {
      stop: message.stopReason, responseId: message.responseId ?? null,
      text: message.content.filter(({ type }) => type === "text").map(({ text }) => text).join(""),
      totalTokens: message.usage.totalTokens, terminal, raw,
      fallback: message.diagnostics?.find(({ type }) => type === "provider_transport_failure")?.details?.fallbackTransport ?? null,
      error: message.errorMessage ?? null,
      retryable: message.stopReason === "error" ? isRetryableAssistantError(message) : null,
    };
  }
  try {
    const first = await call(context("hello"), row.timeoutMs ?? fixture.idleMs, row.holdCallback);
    const firstPosts = posts.length;
    const expires = row.outcome !== "complete";
    const next = expires ? await call(context("hello", "second turn")) : null;
    if (expires) {
      await Promise.race([expiredClose.promise, delay(1_000, undefined, { ref: false })]);
    }
    assert.deepEqual(creates, [{ connection: 1, items: 1, previous: null }],
      JSON.stringify({ name: row.name, transport, connections, first, next, posts, close }));
    assert.equal(connections, 1);
    assert.equal(sawPong, row.traffic === "ping");
    assert.deepEqual(close, expires ? { code: 1000, reason: "idle_timeout" } : null);
    assert.equal(firstPosts, row.outcome === "fallback" ? 1 : 0);
    assert.deepEqual(posts, row.outcome === "fallback"
      ? [{ items: 1, previous: null }, { items: 2, previous: null }]
      : expires ? [{ items: 2, previous: null }] : []);
    assert.deepEqual(first.terminal, [row.outcome === "terminal" ? "error" : "done"]);
    assert.equal(first.responseId, row.outcome === "fallback" ? "resp_sse" : row.outcome === "complete"
      ? "resp_late" : row.prefix === "partial" ? "resp_partial" : null);
    if (expires) assert.equal(next.responseId, "resp_sse");
    return { name: row.name, transport, creates, firstPosts, posts, close, sawPong, first, next };
  } finally {
    closeOpenAICodexWebSocketSessions(sessionId);
    for (const ws of websocket.clients) ws.terminate();
    await new Promise((resolve) => server.close(resolve));
    websocket.close();
  }
}

const observations = [];
for (const row of fixture.cases) {
  for (const transport of ["auto", "websocket-cached", "websocket"]) {
    observations.push(await observe(row, transport));
  }
}
console.log(JSON.stringify(observations));
