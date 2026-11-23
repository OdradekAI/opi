import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) {
  console.error("node observe_openai_choice_usage.mjs <snapshot> <fixture>");
  process.exit(2);
}

const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [{ createModels, createProvider }, chat] = await Promise.all([
  from("packages/ai/dist/models.js"),
  from("packages/ai/dist/api/openai-completions.js"),
]);
const fixture = JSON.parse(await readFile(fixturePath, "utf8"));
const requests = [];
const server = createServer(async (request, response) => {
  let body = "";
  for await (const part of request) body += part;
  assert.equal(request.method, "POST");
  assert.equal(request.url, "/v1/chat/completions");
  assert.equal(request.headers.authorization, "Bearer fixture-key");
  requests.push(JSON.parse(body));
  response.writeHead(200, { "content-type": "text/event-stream" });
  response.end(fixture.sse);
});
await new Promise((listen) => server.listen(0, "127.0.0.1", listen));
try {
  const base = `http://127.0.0.1:${server.address().port}/v1`;
  const model = {
    id: "moonshot-v1", name: "Fixture", api: "openai-completions", provider: "moonshotai",
    baseUrl: base, reasoning: false, input: ["text"], contextWindow: 128000, maxTokens: 4096,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
  const models = createModels();
  models.setProvider(createProvider({
    id: model.provider,
    models: [model],
    api: { stream: chat.stream, streamSimple: chat.streamSimple },
    auth: { apiKey: { name: "Fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } },
  }));
  const result = await models.streamSimple(model, { messages: [{ role: "user", content: "fixture", timestamp: 1 }] }, {
    maxRetries: 0, maxTokens: 4096,
  }).result();
  console.log(JSON.stringify({
    case: fixture.id,
    stop: result.stopReason,
    raw_stop: result.rawStopReason ?? null,
    usage: {
      input: result.usage.input,
      output: result.usage.output,
      cache_read: result.usage.cacheRead,
      cache_write: result.usage.cacheWrite,
      total: result.usage.totalTokens ?? null,
    },
    cost: result.usage.cost.total,
    requests: requests.length,
  }));
} finally {
  server.closeAllConnections();
  await new Promise((closed) => server.close(closed));
}
