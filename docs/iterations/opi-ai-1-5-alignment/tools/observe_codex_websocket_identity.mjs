import assert from "node:assert/strict";
import { once } from "node:events";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_codex_websocket_identity.mjs --snapshot <path>");
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
  "../../../../crates/opi-ai/tests/data/codex-websocket-identity.json", import.meta.url,
), "utf8"));
const context = (...messages) => normalizeContext({
  systemPrompt: "system prompt",
  messages: messages.map((message) => typeof message === "string"
    ? { role: "user", content: message, timestamp: 0 }
    : message),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");

function frame(event) {
  const body = Buffer.from(JSON.stringify(event));
  assert(body.length <= 0xffff);
  const header = Buffer.alloc(body.length <= 125 ? 2 : 4);
  header[0] = 0x81;
  if (header.length === 2) header[1] = body.length;
  else {
    header[1] = 126;
    header.writeUInt16BE(body.length, 2);
  }
  return Buffer.concat([header, body]);
}

async function observe(row, transport) {
  const sessionId = "session-fixed";
  const sent = [];
  let sockets = 0;
  let sse = 0;
  const server = createServer((_request, response) => {
    sse++;
    response.writeHead(500).end();
  });
  const websocket = new WebSocketServer({ noServer: true });
  const replies = [["first"], row.second, ...(row.followUp ? [["third"]] : [])]
    .map((groups) => Buffer.concat(groups.flatMap((group) => fixture.frames[group]).map(frame)));
  server.on("upgrade", (request, socket, head) => {
    assert.equal(request.url, "/codex/responses");
    websocket.handleUpgrade(request, socket, head, (ws) => {
      const connection = ++sockets;
      ws.on("message", (data) => {
        const body = JSON.parse(data);
        assert.equal(body.type, "response.create");
        const reply = replies[sent.length];
        assert(reply, `${row.name}/${transport}: unexpected create`);
        sent.push({ connection, previous: body.previous_response_id ?? null, items: body.input.length });
        // Late frames appear only in the second reply, after its create arrives.
        socket.write(reply);
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
    const terminal = [];
    const source = stream(model, input, {
      apiKey: `fixture.${account}.fixture`, transport, sessionId,
      cacheRetention: "short", reasoningEffort: "high", temperature: 0.2, maxRetries: 0,
      signal: AbortSignal.timeout(4_000),
      onProviderStreamEvent(event) { raw.push(event.type); },
    });
    for await (const event of source) {
      if (event.type === "done" || event.type === "error") terminal.push(event.type);
    }
    const message = await source.result();
    const observation = {
      stop: message.stopReason, responseId: message.responseId ?? null,
      text: message.content.filter(({ type }) => type === "text").map(({ text }) => text).join(""),
      totalTokens: message.usage.totalTokens, endTurn: message.endTurn ?? null, terminal, raw,
    };
    return { observation, message };
  }
  try {
    const first = await call(context("hello"));
    const second = await call(context("hello", "second turn"));
    const third = row.followUp
      ? await call(context("hello", "second turn", second.message, "third turn"))
      : null;
    const cached = transport !== "websocket";
    assert.deepEqual(sent.slice(0, 2), [
      { connection: 1, previous: null, items: 1 },
      { connection: 1, previous: cached ? "resp_first" : null, items: cached ? 1 : 2 },
    ]);
    assert.equal(sent.length, replies.length);
    assert.equal(sse, 0);
    assert.equal(sockets, 1);
    return {
      name: row.name, transport, sockets, sse, sent,
      first: first.observation, second: second.observation, third: third?.observation ?? null,
    };
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
