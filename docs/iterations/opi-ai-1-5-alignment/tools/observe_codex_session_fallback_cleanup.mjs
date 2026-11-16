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
  console.error("node --conditions=source observe_codex_session_fallback_cleanup.mjs --snapshot <path>");
  process.exit(2);
}
const load = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels, cleanupSessionResources }, { openaiCodexProvider }, { WebSocketServer }] = await Promise.all([
  load("packages/ai/dist/index.js"),
  load("packages/ai/dist/providers/openai-codex.js"),
  load("node_modules/ws/wrapper.mjs"),
]);
const { frames, idleMs } = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/codex-websocket-idle-timeout.json", import.meta.url,
), "utf8"));
const sse = frames.sse.map((event) => `data: ${JSON.stringify(event)}\n\n`).join("");
const context = (messages) => ({
  systemPrompt: "system prompt",
  messages: messages.map((content) => ({ role: "user", content, timestamp: 0 })),
  tools: [{ name: "read", description: "Read a file", parameters: { type: "object" } }],
});
const account = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "acct-1" },
})).toString("base64");
const apiKey = `fixture.${account}.fixture`;

function requestFields(body) {
  return {
    cacheKey: body.prompt_cache_key,
    input: body.input.map(({ role, content }) => {
      assert.equal(role, "user");
      if (typeof content === "string") return content;
      return content.map(({ type, text }) => {
        assert.equal(type, "input_text");
        return text;
      }).join("");
    }),
    previous: body.previous_response_id ?? null,
  };
}

async function observe(scope, transport) {
  const global = scope === "global";
  const session = (name) => `cleanup-${scope}-${transport}-${name}`;
  const [target, other, control] = ["target", "other", "control"].map(session);
  const creates = [];
  const posts = [];
  const closes = [];
  const closeWaiters = Array.from({ length: 3 }, () => Promise.withResolvers());
  let connections = 0;
  const server = createServer(async (request, response) => {
    assert.equal(request.method, "POST");
    assert.equal(request.url, "/codex/responses");
    const chunks = [];
    for await (const chunk of request) chunks.push(chunk);
    const data = Buffer.concat(chunks);
    posts.push(requestFields(JSON.parse(request.headers["content-encoding"] === "zstd" ? zstdDecompressSync(data) : data)));
    response.writeHead(200, { "content-type": "text/event-stream", connection: "close" }).end(sse);
  });
  const websocket = new WebSocketServer({ noServer: true });
  server.on("upgrade", (request, socket, head) => {
    assert.equal(request.url, "/codex/responses");
    websocket.handleUpgrade(request, socket, head, (ws) => {
      const connection = ++connections;
      ws.on("close", (code, reason) => {
        if (connection <= 3) {
          closes.push({ connection, code, reason: reason.toString() });
          closeWaiters[connection - 1].resolve();
        }
      });
      ws.on("message", (data) => {
        const body = JSON.parse(data);
        assert.equal(body.type, "response.create");
        creates.push({ connection, body: requestFields(body) });
        for (const event of connection === 1 ? frames.none : connection === 2 ? frames.partial : frames.next) {
          ws.send(JSON.stringify(event));
        }
      });
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  const models = createModels();
  models.setProvider({
    ...openaiCodexProvider(), baseUrl,
    auth: { apiKey: { name: "Loopback bearer", async resolve() { return { auth: { apiKey, baseUrl } }; } } },
  });
  const model = {
    id: "gpt-5.4", name: "GPT-5.4", api: "openai-codex-responses", provider: "openai-codex",
    baseUrl, reasoning: true, input: ["text", "image"], contextWindow: 272000, maxTokens: 128000,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
  async function call(step, sessionId, input, timeoutMs) {
    const createsBefore = creates.length;
    const postsBefore = posts.length;
    const raw = [];
    const terminal = [];
    const source = models.streamSimple(model, context(input), {
      transport, sessionId, timeoutMs,
      cacheRetention: "short", reasoning: "high", temperature: 0.2, maxTokens: 1024, maxRetries: 0,
      signal: AbortSignal.timeout(4_000),
      onProviderStreamEvent(event) { raw.push(event.type); },
    });
    for await (const event of source) {
      if (event.type === "done" || event.type === "error") terminal.push(event.type);
    }
    const message = await source.result();
    return {
      step, creates: creates.slice(createsBefore), posts: posts.slice(postsBefore),
      stop: message.stopReason, responseId: message.responseId ?? null,
      text: message.content.filter(({ type }) => type === "text").map(({ text }) => text).join(""),
      totalTokens: message.usage.totalTokens, terminal, raw, error: message.errorMessage ?? null,
    };
  }

  try {
    const first = ["hello"];
    const full = ["hello", "second turn"];
    const calls = [
      await call("targetSeed", target, first, idleMs),
      await call("otherSeed", other, first, idleMs),
      await call("targetSticky", target, full),
      await call("otherSticky", other, full),
      await call("controlWarm", control, first),
    ];
    const selector = global ? undefined : target;
    assert.equal(cleanupSessionResources(selector), undefined);
    calls.push(
      await call("controlAfter", control, full),
      await call("targetAfter", target, full),
      await call("otherAfter", other, full),
    );
    await Promise.race([
      Promise.all(closeWaiters.slice(0, global ? 3 : 2).map(({ promise }) => promise)),
      delay(1_000, undefined, { ref: false }).then(() => {
        throw new Error(`expected peer Close: ${JSON.stringify({ scope, transport, connections, closes, calls })}`);
      }),
    ]);
    assert.equal(connections, global ? 4 : 3);
    const expectedCloses = [
      { connection: 1, code: 1000, reason: "idle_timeout" },
      { connection: 2, code: 1000, reason: "idle_timeout" },
    ];
    if (global) expectedCloses.push({ connection: 3, code: 1000, reason: "debug_close" });
    assert.deepEqual(closes, expectedCloses);

    const body = (cacheKey, input, previous = null) => ({ cacheKey, input, previous });
    const ws = (connection, body) => ({ connection, body });
    const controlBody = !global && transport !== "websocket"
      ? body(control, ["second turn"], "resp_next_ws") : body(control, full);
    const expected = [
      [[ws(1, body(target, first))], [body(target, first)], "resp_sse", 3],
      [[ws(2, body(other, first))], [], "resp_partial", 0],
      [[], [body(target, full)], "resp_sse", 3],
      [[], [body(other, full)], "resp_sse", 3],
      [[ws(3, body(control, first))], [], "resp_next_ws", 7],
      [[ws(global ? 4 : 3, controlBody)], [], "resp_next_ws", 7],
      [[], [body(target, full)], "resp_sse", 3],
      [[], [body(other, full)], "resp_sse", 3],
    ];
    for (const [index, [creates, posts, responseId, totalTokens]] of expected.entries()) {
      const failure = totalTokens === 0;
      const { error, ...call } = calls[index];
      assert.deepEqual(call, {
        step: calls[index].step, creates, posts, responseId, totalTokens,
        stop: failure ? "error" : "stop", terminal: [failure ? "error" : "done"],
        text: failure ? "partial answer" : "",
        raw: failure ? frames.partial.map(({ type }) => type) : ["response.done"],
      });
      if (failure) assert.match(error, /idle timeout/i);
      else assert.equal(error, null);
    }
    return { scope, transport, selector: selector ?? null, calls, connections, closes: [...closes], cleanup: null };
  } finally {
    cleanupSessionResources();
    for (const ws of websocket.clients) ws.terminate();
    await new Promise((done) => server.close(done));
    websocket.close();
  }
}

const observations = [];
for (const scope of ["target", "global"]) {
  for (const transport of ["auto", "websocket-cached", "websocket"]) {
    observations.push(await observe(scope, transport));
  }
}
console.log(JSON.stringify(observations));
