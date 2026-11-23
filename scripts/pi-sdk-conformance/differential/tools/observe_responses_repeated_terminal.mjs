import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) {
  console.error("node --conditions=source observe_responses_repeated_terminal.mjs <snapshot> <fixture>");
  process.exit(2);
}

const fixture = JSON.parse(await readFile(resolve(fixturePath), "utf8"));
const { stream } = await import(pathToFileURL(resolve(snapshot, "packages/ai/src/api/openai-responses.ts")).href);
const { normalizeContext } = await import(pathToFileURL(resolve(snapshot, "packages/ai/src/utils/transcript.ts")).href);

const body = fixture.events
  .map((event) => `event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`)
  .join("");
const model = {
  id: "gpt-4o",
  name: "Fixture",
  api: "openai-responses",
  provider: "openai",
  baseUrl: "https://responses.fixture.invalid/v1",
  reasoning: false,
  input: ["text"],
  contextWindow: 128000,
  maxTokens: 16384,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
};
const context = normalizeContext({
  systemPrompt: "",
  messages: [{ role: "user", content: "hello", timestamp: 0 }],
});
const providerEvents = [];
const eventTypes = [];
const streamResult = stream(model, context, {
  apiKey: "fixture-key",
  fetch: async (input, init) => {
    assert.equal(new URL(input instanceof Request ? input.url : input).hostname, "responses.fixture.invalid");
    assert.equal(init?.method ?? input.method, "POST");
    return new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } });
  },
  onProviderStreamEvent: async (event) => {
    providerEvents.push(event.type);
  },
});

let doneCount = 0;
for await (const event of streamResult) {
  eventTypes.push(event.type);
  if (event.type === "done") doneCount++;
}
const result = await streamResult.result();
console.log(JSON.stringify({
  case: fixture.id,
  stop: result.stopReason,
  raw_stop: result.rawStopReason ?? null,
  usage: { input: result.usage.input, output: result.usage.output, total: result.usage.totalTokens },
  done_count: doneCount,
  event_types: eventTypes,
  provider_events: providerEvents,
}));
