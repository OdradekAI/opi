import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { pathToFileURL } from "node:url";

const [snapshot, fixture, rustTrace] = process.argv.slice(2);
if (!snapshot || !fixture || !rustTrace || process.argv.length !== 5) {
  console.error("node observe_chat_argument_preview.mjs <frozen-pi> <fixture> <rust-trace>");
  process.exit(2);
}
const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels, createProvider }, chat, { parseStreamingJson },
  { ToolCallArgumentsStream }, { PartialChanges }, { track }] = await Promise.all([
  from("packages/ai/dist/models.js"), from("packages/ai/dist/api/openai-completions.js"),
  from("packages/ai/dist/utils/json-parse.js"), from("packages/ai/dist/utils/streaming-json.js"),
  from("packages/durable/src/harness/partial-changes.ts"),
  from("packages/chord/src/delta/index.ts"),
]);

async function until(promise, name) {
  const controller = new AbortController();
  try {
    return await Promise.race([promise, sleep(5000, undefined, { signal: controller.signal })
      .then(() => assert.fail("no progress: " + name))]);
  } finally { controller.abort(); }
}

// Exercise Durable's actual tracked commit, supplying either SDK's preview.
// This is a component control, not a full Harness/task-settlement run.
function durable(pieces, previews, final) {
  const changes = new PartialChanges();
  const tracker = track({ generation: { attempt: 1 } });
  const partial = {
    role: "assistant", api: "openai-completions", provider: "openai", model: "gpt-4o",
    content: [{ type: "toolCall", id: "call_preview", name: "record", arguments: {} }],
    usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } },
    stopReason: "pending", timestamp: 1,
  };
  const commit = () => {
    const message = structuredClone(partial);
    const taken = changes.take(message);
    const change = tracker.beginChange();
    changes.commit(change.state.generation, message, taken);
    tracker.adopt(change.prepare());
    return structuredClone(tracker.value.generation.message.content[0].arguments);
  };
  changes.observe({ type: "toolcall_start", contentIndex: 0, partial });
  const progress = pieces.map((delta, index) => {
    partial.content[0].arguments = structuredClone(previews[index]);
    changes.observe({ type: "toolcall_delta", contentIndex: 0, delta, partial });
    return commit();
  });
  partial.content[0].arguments = structuredClone(final);
  changes.observe({ type: "toolcall_end", contentIndex: 0, toolCall: partial.content[0], partial });
  return { progress, final: commit() };
}

async function observe(test, rust) {
  const frames = test.pieces.map((part, index) => ({ id: "preview", choices: [{ index: 0,
    delta: { tool_calls: [{ index: 0, ...(index === 0 ? { id: "call_preview", type: "function" } : {}),
      function: { ...(index === 0 ? { name: "record" } : {}), arguments: part } }] }, finish_reason: null }] }));
  frames.push({ id: "preview", choices: [{ index: 0, delta: {}, finish_reason: "tool_calls" }] });
  const requests = [];
  const server = createServer(async (request, response) => {
    try {
      let body = "";
      for await (const part of request) body += part;
      assert.equal(request.method, "POST");
      assert.equal(request.url, "/v1/chat/completions");
      assert.equal(request.headers.authorization, "Bearer fixture-key");
      requests.push(JSON.parse(body));
      response.writeHead(200, { "content-type": "text/event-stream" });
      response.end(frames.map((frame) => `data: ${JSON.stringify(frame)}\n\n`).join("") + "data: [DONE]\n\n");
    } catch (error) { response.destroy(error); }
  });
  await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
  const gates = frames.map(() => ({ reached: Promise.withResolvers(), release: Promise.withResolvers() }));
  try {
    const model = { id: "gpt-4o", name: "Fixture", api: "openai-completions", provider: "openai",
      baseUrl: `http://127.0.0.1:${server.address().port}/v1`, reasoning: false, input: ["text"],
      contextWindow: 128000, maxTokens: 4096, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 } };
    const models = createModels();
    models.setProvider(createProvider({ id: "openai", models: [model],
      api: { stream: chat.stream, streamSimple: chat.streamSimple },
      auth: { apiKey: { name: "Fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } } }));
    let seen = 0;
    const call = models.streamSimple(model, { messages: [{ role: "user", content: "fixture", timestamp: 1 }],
      tools: [{ name: "record", description: "Records fixture arguments", parameters: { type: "object", properties: {} } }] }, {
      maxRetries: 0, maxTokens: 4096,
      async onProviderStreamEvent(chunk) {
        const index = seen++;
        assert.deepEqual(chunk, frames[index]);
        gates[index].reached.resolve();
        await gates[index].release.promise;
      },
    });
    const previews = [];
    let ends = 0;
    let terminal;
    const collect = (async () => {
      for await (const event of call) {
        assert.notEqual(event.type, "error", event.error?.errorMessage);
        if (event.type === "toolcall_delta") {
          assert.equal(event.contentIndex, 0);
          assert.equal(event.delta, test.pieces[previews.length]);
          previews.push(structuredClone(event.partial.content[0].arguments));
        }
        if (event.type === "toolcall_end") ends++;
        if (event.type === "done") { assert.equal(terminal, undefined); terminal = event.message; }
      }
    })();
    for (let index = 0; index < test.pieces.length; index++) {
      await until(gates[index].reached.promise, "frame");
      gates[index].release.resolve();
      await until(gates[index + 1].reached.promise, "next frame");
      await new Promise(setImmediate);
      assert.equal(previews.length, index + 1);
      assert.equal(ends, 0);
      assert.equal(terminal, undefined);
    }
    gates.at(-1).release.resolve();
    await until(collect, "EOF");
    const result = await call.result();
    assert.equal(ends, 1);
    assert.deepEqual(result, terminal);
    assert.equal(result.content[0].argumentsError, undefined);
    assert.deepEqual(result.content[0].arguments, test.final);
    assert.equal(requests.length, 1);
    const parser = new ToolCallArgumentsStream();
    assert.deepEqual(test.pieces.map((delta) => structuredClone(parser.append(delta))), previews);
    let raw = "";
    const utility = test.pieces.map((delta) => parseStreamingJson(raw += delta));
    return { name: test.name, previews, utility, final: result.content[0].arguments, request: requests[0],
      durable: { pi: durable(test.pieces, previews, test.final),
        opi: durable(test.pieces, rust.previews.map((row) => row.preview), rust.final) } };
  } finally {
    for (const gate of gates) gate.release.resolve();
    server.closeAllConnections();
    await new Promise((closed) => server.close(closed));
  }
}

const cases = JSON.parse(await readFile(fixture, "utf8"));
const rust = JSON.parse(await readFile(rustTrace, "utf8"));
assert.deepEqual(rust.cases.map((row) => row.name), cases.map((row) => row.name));
const traces = [];
for (const [index, test] of cases.entries()) traces.push(await observe(test, rust.cases[index]));
console.log(JSON.stringify({ cases: traces }));
