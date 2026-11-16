import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [snapshot, fixturePath] = process.argv.slice(2);
if (!snapshot || !fixturePath || process.argv.length !== 4) {
  console.error("node observe_bedrock_repeated_metadata.mjs <snapshot> <fixture>");
  process.exit(2);
}

const fixture = JSON.parse(await readFile(resolve(fixturePath), "utf8"));
globalThis.__opiBedrockFixture = fixture;

const modulePath = resolve(snapshot, "packages", "ai", "dist", "api", "bedrock-converse-stream.js");
assert(modulePath.includes("packages"));
const { stream } = await import(pathToFileURL(modulePath).href);

const model = {
  id: "anthropic.claude-sonnet-4",
  name: "Fixture",
  api: "bedrock-converse-stream",
  provider: "amazon-bedrock",
  baseUrl: "https://bedrock-runtime.us-east-1.amazonaws.com",
  reasoning: false,
  input: ["text"],
  contextWindow: 128000,
  maxTokens: 4096,
  cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
};
const context = {
  messages: [{ role: "user", content: "fixture", timestamp: 1 }],
};
const providerEvents = [];
const events = [];
const eventStream = stream(model, context, {
  apiKey: "fixture-key",
  maxTokens: 4096,
  onProviderStreamEvent: async (event) => {
    providerEvents.push(Object.keys(event)[0] ?? "unknown");
  },
});
for await (const event of eventStream) {
  events.push(event.type);
}
const result = await eventStream.result();
assert.equal(result.stopReason, fixture.expected.stop);
assert.equal(result.rawStopReason, fixture.expected.raw_stop);
assert.equal(result.usage.input, fixture.expected.input_tokens);
assert.equal(result.usage.output, fixture.expected.output_tokens);
assert.equal(result.usage.cacheRead, fixture.expected.cache_read_tokens);

console.log(JSON.stringify({
  case: fixture.id,
  stop: result.stopReason,
  raw_stop: result.rawStopReason ?? null,
  usage: {
    input: result.usage.input,
    output: result.usage.output,
    cache_read: result.usage.cacheRead,
  },
  done_count: events.filter((type) => type === "done").length,
  events,
  provider_events: providerEvents,
}));
