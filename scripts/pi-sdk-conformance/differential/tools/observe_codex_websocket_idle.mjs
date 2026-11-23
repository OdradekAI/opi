import assert from "node:assert/strict";
import { once } from "node:events";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_websocket_idle.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ stream, closeOpenAICodexWebSocketSessions }, { normalizeContext }, { WebSocketServer }] =
  await Promise.all([
    load("packages/ai/src/api/openai-codex-responses.ts"),
    load("packages/ai/src/utils/transcript.ts"),
    load("node_modules/ws/wrapper.mjs"),
  ]);
const fixture = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-websocket-idle.json", import.meta.url,
), "utf8"));
const context = (...messages) => normalizeContext({
  systemPrompt: "system prompt",
  messages: messages.map((content) => ({ role: "user", content, timestamp: 0 })),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");

function frame(opcode, payload) {
  const body = Buffer.from(payload);
  assert(body.length <= 0xffff);
  const header = Buffer.alloc(body.length <= 125 ? 2 : 4);
  header[0] = 0x80 | opcode;
  if (header.length === 2) header[1] = body.length;
  else {
    header[1] = 126;
    header.writeUInt16BE(body.length, 2);
  }
  return Buffer.concat([header, body]);
}
const eventFrame = (event) => frame(1, JSON.stringify(event));

async function observe(row, transport) {
  const sessionId = "session-fixed";
  const sent = [];
  const clientFrames = [];
  let sockets = 0;
  let sse = 0;
  const server = createServer((_request, response) => {
    sse++;
    response.writeHead(500).end();
  });
  const websocket = new WebSocketServer({ noServer: true });
  server.on("upgrade", (request, socket, head) => {
    assert.equal(request.url, "/codex/responses");
    websocket.handleUpgrade(request, socket, head, (ws) => {
      const connection = ++sockets;
      ws.on("pong", (data) => clientFrames.push(`pong:${data}`));
      ws.on("message", (data) => {
        const body = JSON.parse(data);
        assert.equal(body.type, "response.create");
        sent.push({
          connection, previous: body.previous_response_id ?? null, items: body.input.length,
        });
        clientFrames.push("create");
        const frames = sent.length === 1
          ? [
            eventFrame(fixture.first),
            ...row.tail.map((name) => name === "ping"
              ? frame(9, "idle") : eventFrame(fixture.late[name])),
          ]
          : fixture.second.map(eventFrame);
        socket.write(Buffer.concat(frames));
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
  async function call(input) {
    const raw = [];
    const events = [];
    const source = stream(model, input, {
      apiKey: `fixture.${account}.fixture`, transport, sessionId,
      cacheRetention: "short", reasoningEffort: "high", temperature: 0.2, maxRetries: 0,
      signal: AbortSignal.timeout(4_000),
      onProviderStreamEvent(event) { raw.push(event.type); },
    });
    for await (const event of source) events.push(event.type);
    const message = await source.result();
    return {
      stop: message.stopReason, responseId: message.responseId ?? null,
      text: message.content.filter(({ type }) => type === "text").map(({ text }) => text).join(""),
      totalTokens: message.usage.totalTokens, endTurn: message.endTurn ?? null,
      terminal: events.filter((type) => type === "done" || type === "error"),
      raw,
    };
  }
  try {
    const first = await call(context("hello"));
    const second = await call(context("hello", "second turn"));
    assert.deepEqual(first, {
      stop: "stop", responseId: "resp_first", text: "", totalTokens: 5, endTurn: false,
      terminal: ["done"], raw: ["response.done"],
    }, row.name);
    assert.deepEqual(second, {
      stop: "stop", responseId: "resp_second", text: "second answer", totalTokens: 8, endTurn: true,
      terminal: ["done"], raw: fixture.second.map(({ type }) => type),
    }, row.name);
    const cached = transport !== "websocket";
    assert.deepEqual(sent, [
      { connection: 1, previous: null, items: 1 },
      { connection: 1, previous: cached ? "resp_first" : null, items: cached ? 1 : 2 },
    ]);
    assert.equal(sse, 0);
    assert.equal(sockets, 1);
    assert.deepEqual(clientFrames, row.tail.includes("ping")
      ? ["create", "pong:idle", "create"] : ["create", "create"]);
    return { name: row.name, transport, sockets, sse, sent, clientFrames, first, second };
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
