import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_responses_error.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/src", path)).href);
const [{ stream: responses }, { stream: azure }, { stream: codex },
  { normalizeContext }, { isContextOverflow }, { isRetryableAssistantError }] = await Promise.all([
  fromSnapshot("api/openai-responses.ts"),
  fromSnapshot("api/azure-openai-responses.ts"),
  fromSnapshot("api/openai-codex-responses.ts"),
  fromSnapshot("utils/transcript.ts"),
  fromSnapshot("utils/overflow.ts"),
  fromSnapshot("utils/retry.ts"),
]);
const fixtures = new URL("../../../../crates/opi-ai/tests/data/", import.meta.url);
const cases = JSON.parse(await readFile(new URL("responses-error.json", fixtures), "utf8"));
const frames = JSON.parse(await readFile(new URL("responses-failed.json", fixtures), "utf8"));
const context = normalizeContext({ messages: [{ role: "user", content: "Hello", timestamp: 0 }] });
const codexPayload = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "fixture-account" },
})).toString("base64url");
const baseUrl = "https://responses.fixture.invalid/v1";

async function observe(row, stream, api, provider) {
  const body = frames.prefix.map((event) => `data: ${JSON.stringify(event)}\n\n`).join("")
    + (row.event ? `event: ${row.event}\n` : "") + `data: ${JSON.stringify(row.data)}\n\n`
    + `data: ${JSON.stringify(frames.afterFailure)}\n\n`;
  const model = {
    id: "gpt-5-mini", name: "Fixture", api, provider, baseUrl,
    reasoning: true, input: ["text"], contextWindow: 400000, maxTokens: 128000,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
  let sends = 0;
  const resultStream = stream(model, context, {
    apiKey: provider === "openai-codex" ? `fixture.${codexPayload}.fixture` : "fixture-key",
    azureBaseUrl: baseUrl,
    transport: "sse",
    maxRetries: 1,
    fetch: async (input, init) => {
      assert.equal(new URL(input instanceof Request ? input.url : input).hostname, "responses.fixture.invalid");
      assert.equal(init?.method ?? input.method, "POST");
      sends++;
      return new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } });
    },
  });
  let terminals = 0;
  for await (const event of resultStream) {
    if (event.type === "error" || event.type === "done") terminals++;
  }
  const message = await resultStream.result();
  const partial = message.content.filter((block) => block.type === "text").map((block) => block.text).join("");
  const overflow = isContextOverflow(message);
  const retryable = !overflow && isRetryableAssistantError(message);
  const expected = row.piDecision ?? row.decision;
  assert.equal(message.stopReason, "error", row.name);
  assert.equal(message.rawStopReason, undefined, row.name);
  assert.equal(partial, "partial", row.name);
  assert.equal(message.responseId, "resp_partial", row.name);
  assert.equal(message.usage.totalTokens, 0, row.name);
  assert.equal(sends, 1, row.name);
  assert.equal(terminals, 1, row.name);
  assert.equal(retryable, expected === "transient", row.name);
  assert.equal(overflow, expected === "overflow", row.name);
  return {
    name: row.name, api, expected, diagnostic: message.errorMessage, retryable, overflow,
    partial, rawStop: message.rawStopReason ?? null, stop: message.stopReason,
    sends, terminals, responseId: message.responseId, totalTokens: message.usage.totalTokens,
  };
}

const observations = [];
for (const row of cases) {
  if (row.route === "responses") {
    observations.push(await observe(row, responses, "openai-responses", "openai"));
  } else {
    assert.equal(row.route, "codex");
    observations.push(await observe(row, codex, "openai-codex-responses", "openai-codex"));
  }
  if (row.alsoAzure) {
    observations.push(await observe(row, azure, "azure-openai-responses", "azure"));
  }
}
console.log(JSON.stringify(observations));
