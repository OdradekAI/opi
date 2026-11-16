import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) process.exit(2);
const load = (p) => import(pathToFileURL(resolve(snapshot, p)).href);
const [{ createModels, createProvider }, api] = await Promise.all([
  load("packages/ai/dist/models.js"),
  load("packages/ai/dist/api/google-generative-ai.js"),
]);
const fixture = JSON.parse(await readFile(resolve(fixturePath), "utf8"));
const body = fixture.chunks.map((chunk) => `data: ${JSON.stringify(chunk)}\n\n`).join("");

async function run(route) {
  let requests = 0;
  const server = createServer(async (_request, response) => {
    requests++;
    response.writeHead(200, { "content-type": "text/event-stream" });
    response.end(body);
  });
  await new Promise((done) => server.listen(0, "127.0.0.1", done));
  try {
    const model = {
      id: "gemini-2.5-flash",
      name: "Repeated Gemini finish fixture",
      provider: "google",
      api: "google-generative-ai",
      baseUrl: `http://127.0.0.1:${server.address().port}`,
      reasoning: false,
      input: ["text"],
      contextWindow: 100000,
      maxTokens: 4096,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    };
    let sourceCall;
    const mappedApi = {
      stream: (...args) => (sourceCall = api.stream(...args)),
      streamSimple: (...args) => (sourceCall = api.streamSimple(...args)),
    };
    const models = createModels();
    models.setProvider(createProvider({
      id: "google",
      models: [model],
      api: mappedApi,
      auth: { apiKey: { name: "fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } },
    }));
    const providerEvents = [];
    const options = {
      apiKey: "fixture-key",
      maxTokens: 4096,
      maxRetries: 0,
      onProviderStreamEvent: async (event) => providerEvents.push({
        finish: event.candidates?.[0]?.finishReason ?? null,
        output: event.usageMetadata?.candidatesTokenCount ?? null,
      }),
    };
    const context = { messages: [{ role: "user", content: [{ type: "text", text: "hello" }] }] };
    const call = route === "direct"
      ? mappedApi.stream(model, context, options)
      : models.streamSimple(model, context, options);
    const eventTypes = [];
    let doneCount = 0;
    for await (const event of call) {
      eventTypes.push(event.type);
      if (event.type === "done") doneCount++;
    }
    const result = await call.result();
    if (sourceCall) await sourceCall.result();
    return {
      route,
      stop: result.stopReason,
      raw_stop: result.rawStopReason ?? null,
      error: result.errorMessage ?? null,
      usage: { input: result.usage.input, output: result.usage.output },
      done_count: doneCount,
      event_types: eventTypes,
      provider_events: providerEvents,
      requests,
    };
  } finally {
    server.closeAllConnections();
    await new Promise((done) => server.close(done));
  }
}

const cases = [];
for (const route of ["simple", "direct"]) cases.push(await run(route));
const expectedProviderEvents = fixture.chunks.map((chunk) => ({
  finish: chunk.candidates?.[0]?.finishReason ?? null,
  output: chunk.usageMetadata?.candidatesTokenCount ?? null,
}));
assert.deepEqual(cases.map((item) => item.provider_events), [expectedProviderEvents, expectedProviderEvents]);
console.log(JSON.stringify({ case: fixture.id, cases }));
