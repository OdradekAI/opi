import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_responses_failed.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/src", path)).href);
const [
  { stream: openai },
  { stream: azure },
  { stream: codex },
  { normalizeContext },
  { isContextOverflow },
  { isRetryableAssistantError },
] = await Promise.all([
  fromSnapshot("api/openai-responses.ts"),
  fromSnapshot("api/azure-openai-responses.ts"),
  fromSnapshot("api/openai-codex-responses.ts"),
  fromSnapshot("utils/transcript.ts"),
  fromSnapshot("utils/overflow.ts"),
  fromSnapshot("utils/retry.ts"),
]);
const fixture = JSON.parse(await readFile(new URL(
  "../../../../crates/opi-ai/tests/data/responses-failed.json", import.meta.url,
), "utf8"));
const context = normalizeContext({
  systemPrompt: "",
  messages: [{ role: "user", content: "Hello", timestamp: 0 }],
});
const codexPayload = Buffer.from(JSON.stringify({
  "https://api.openai.com/auth": { chatgpt_account_id: "fixture-account" },
})).toString("base64url");
const baseUrl = "https://responses.fixture.invalid/v1";

async function observe(stream, api, provider, row, expected) {
  const terminal = {
    type: "response.failed",
    response: { status: "failed", error: row.error, usage: { input_tokens: 21, output_tokens: 9 } },
  };
  const body = [...fixture.prefix, terminal, fixture.afterFailure]
    .map((event) => `event: ${event.type}\ndata: ${JSON.stringify(event)}\n\n`).join("");
  const model = {
    id: "gpt-5-mini", name: "Fixture", api, provider, baseUrl,
    reasoning: true, input: ["text"], contextWindow: 400000, maxTokens: 128000,
    cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
  let sends = 0;
  const streamResult = stream(model, context, {
    apiKey: provider === "openai-codex" ? `fixture.${codexPayload}.fixture` : "fixture-key",
    transport: "sse",
    maxRetries: 1,
    azureBaseUrl: baseUrl,
    fetch: async (input, init) => {
      assert.equal(new URL(input instanceof Request ? input.url : input).hostname, "responses.fixture.invalid");
      assert.equal(init?.method ?? input.method, "POST");
      sends++;
      return new Response(body, { status: 200, headers: { "content-type": "text/event-stream" } });
    },
  });
  let terminals = 0;
  for await (const event of streamResult) {
    if (event.type === "error" || event.type === "done") terminals++;
  }
  const result = await streamResult.result();
  const partial = result.content.filter((block) => block.type === "text").map((block) => block.text).join("");
  const overflow = isContextOverflow(result);
  const pattern = isRetryableAssistantError(result);
  const retryable = !overflow && pattern;
  assert.equal(result.stopReason, "error", row.name);
  assert.equal(result.rawStopReason, provider === "openai-codex" ? undefined : "failed", row.name);
  assert.equal(partial, "partial", row.name);
  assert.equal(result.responseId, "resp_partial", row.name);
  assert.equal(result.usage.totalTokens, 0, row.name);
  assert.equal(terminals, 1, row.name);
  assert.equal(sends, 1, row.name);
  assert.equal(retryable, expected === "transient", row.name);
  assert.equal(overflow, expected === "overflow", row.name);
  return {
    api, name: row.name, decision: expected, stop: result.stopReason, rawStop: result.rawStopReason ?? null,
    partial, responseId: result.responseId, totalTokens: result.usage.totalTokens, terminals, sends,
    diagnostic: result.errorMessage, pattern, overflow, retryable,
  };
}

const observations = [];
for (const row of fixture.cases) {
  observations.push(await observe(openai, "openai-responses", "openai", row, row.decision));
}
const codeOnly = fixture.cases.find((row) => row.name === "gateway-read-code");
const messageOnly = fixture.cases.find((row) => row.name === "gateway-http2");
observations.push(await observe(azure, "azure-openai-responses", "azure", codeOnly, "transient"));
observations.push(await observe(codex, "openai-codex-responses", "openai-codex", codeOnly, "terminal"));
observations.push(await observe(codex, "openai-codex-responses", "openai-codex", messageOnly, "transient"));
console.log(JSON.stringify(observations));
