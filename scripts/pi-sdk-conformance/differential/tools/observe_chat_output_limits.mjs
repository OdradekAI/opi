import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) {
  console.error("node observe_chat_output_limits.mjs <snapshot> <fixture>");
  process.exit(2);
}

const from = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/dist", path)).href);
const [{ createModels, createProvider }, chat, { normalizeContext }, { estimateContextTokens }] = await Promise.all([
  from("models.js"), from("api/openai-completions.js"), from("utils/transcript.js"), from("utils/estimate.js"),
]);
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
const cost = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 };
const requests = [];
const server = createServer(async (request, response) => {
  try {
    let data = "";
    for await (const chunk of request) data += chunk;
    const body = JSON.parse(data);
    requests.push({ method: request.method, path: request.url, authorization: request.headers.authorization, body });
    const stop = fixture.fields.some((field) => body[field] === 1) ? "length" : "stop";
    response.writeHead(200, { "content-type": "text/event-stream" });
    response.end(fixture.sse[stop]);
  } catch (error) {
    response.destroy(error);
  }
});
await new Promise((listen) => server.listen(0, "127.0.0.1", listen));
try {
  const rows = [];
  for (const field of fixture.fields) {
    const model = {
      id: fixture.model.id, name: fixture.model.name, provider: "openai", api: "openai-completions",
      baseUrl: "http://127.0.0.1:" + server.address().port + "/v1", input: ["text"], reasoning: false,
      contextWindow: fixture.model.context_window, maxTokens: fixture.model.max_tokens, cost,
      compat: { maxTokensField: field, supportsStore: true, supportsUsageInStreaming: true },
    };
    const models = createModels();
    let resolves = 0;
    models.setProvider(createProvider({
      id: model.provider, models: [model], api: { stream: chat.stream, streamSimple: chat.streamSimple },
      auth: { apiKey: { name: "Fixture", resolve: async () => {
        resolves++;
        return { auth: { apiKey: "fixture-route-key" } };
      } } },
    }));
    for (const row of fixture.cases) {
      const messages = [{ role: "user", content: fixture.input, timestamp: 3 }];
      if (row.anchor !== null) {
        messages.unshift(
          { role: "user", content: fixture.history[0], timestamp: 1 },
          {
            role: "assistant", content: [{ type: "text", text: fixture.history[1] }],
            api: model.api, provider: model.provider, model: model.id, timestamp: 2, stopReason: "stop",
            usage: { ...cost, input: row.anchor - 1, output: 1, totalTokens: row.anchor, cost: { ...cost, total: 0 } },
          },
        );
      }
      const context = { messages };
      const transcript = normalizeContext(context);
      const estimate = estimateContextTokens(transcript).tokens;
      for (const entry of ["native", "simple", "direct"]) {
        const options = { cacheRetention: "none", maxRetries: 0, timeoutMs: 5000 };
        if (row.max_tokens !== null) options.maxTokens = row.max_tokens;
        const before = resolves;
        const sent = requests.length;
        const call = entry === "direct" ? chat.stream(model, transcript, { ...options, apiKey: "fixture-direct-key" })
          : entry === "native" ? models.stream(model, context, options)
            : models.streamSimple(model, context, options);
        const message = await call.result();
        assert.ok(message.stopReason === "stop" || message.stopReason === "length", message.errorMessage);
        assert.equal(message.content.length, 1);
        assert.equal(message.content[0].type, "text");
        assert.equal(requests.length, sent + 1, field + "/" + row.id + "/" + entry);
        rows.push({
          field, case: row.id, entry, estimate, resolver_calls: resolves - before, request: requests[sent],
          result: {
            text: message.content[0].text, stop: message.stopReason, raw_stop: message.rawStopReason ?? null,
            response_id: message.responseId ?? null, input_tokens: message.usage.input, output_tokens: message.usage.output,
            total_tokens: message.usage.totalTokens, error: message.errorMessage ?? null,
          },
        });
      }
    }
  }
    console.log(JSON.stringify(rows));
} finally {
  server.closeAllConnections();
  await new Promise((closed) => server.close(closed));
}
