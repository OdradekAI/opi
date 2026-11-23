import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setTimeout as sleep } from "node:timers/promises";
import { pathToFileURL } from "node:url";

const [snapshot, fixture, beforeTrace, afterTrace] = process.argv.slice(2);
if (!snapshot || !fixture || process.argv.length > 6 || Boolean(beforeTrace) !== Boolean(afterTrace)) {
  console.error("node observe_chat_interleaving.mjs <snapshot> <sse-fixture> [<before-trace> <after-trace>]");
  process.exit(2);
}

const from = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { createModels, createProvider },
  chat,
  anthropic,
  { fauxAssistantMessage, fauxText },
] = await Promise.all([
  from("packages/ai/dist/models.js"),
  from("packages/ai/dist/api/openai-completions.js"),
  from("packages/ai/dist/api/anthropic-messages.js"),
  from("packages/ai/dist/providers/faux.js"),
]);

const user = { role: "user", content: "fixture", timestamp: 1 };
const tools = [{
  name: "record", description: "Records fixture arguments",
  parameters: { type: "object", properties: { a: { type: "string" }, n: { type: "integer" } }, required: ["a", "n"] },
}];
const toolResult = {
  role: "toolResult", toolCallId: "call_interleaved", toolName: "record",
  content: [{ type: "text", text: "ok" }], isError: false, timestamp: 1,
};

function model(id, api, provider, baseUrl) {
  return {
    id, name: "Fixture", api, provider, baseUrl, reasoning: false, input: ["text"],
    contextWindow: 128000, maxTokens: 4096, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
  };
}

function view(message) {
  return {
    content: structuredClone(message.content), stop: message.stopReason.replace("toolUse", "tool_use"),
    raw_stop: message.rawStopReason ?? null, response_id: message.responseId ?? null,
    input_tokens: message.usage.input, output_tokens: message.usage.output,
  };
}

function eventView(event) {
  return {
    kind: event.type, index: event.contentIndex ?? null, delta: event.delta ?? null,
    content: event.content ?? null, message: view(event.partial ?? event.message),
  };
}

async function until(promise, checkpoint) {
  const controller = new AbortController();
  try {
    return await Promise.race([
      promise,
      sleep(5000, undefined, { signal: controller.signal }).then(() => assert.fail("no progress at " + checkpoint)),
    ]);
  } finally {
    controller.abort();
  }
}

async function observe(sse) {
  const frames = sse.trimEnd().split(/\r?\n\r?\n/).map((line) => line.slice("data: ".length));
  assert.equal(frames.pop(), "[DONE]");
  const chunks = frames.map(JSON.parse);
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
      response.end(sse);
    } catch (error) {
      response.destroy(error);
    }
  });
  await new Promise((listen) => server.listen(0, "127.0.0.1", listen));
  try {
    const base = "http://127.0.0.1:" + server.address().port;
    const gpt = model("gpt-4o", "openai-completions", "openai", base + "/v1");
    const models = createModels();
    models.setProvider(createProvider({
      id: gpt.provider, models: [gpt], api: { stream: chat.stream, streamSimple: chat.streamSimple },
      auth: { apiKey: { name: "Fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } },
    }));
    const gates = chunks.map(() => ({ reached: Promise.withResolvers(), release: Promise.withResolvers() }));
    let seen = 0;
    const call = models.streamSimple(gpt, { messages: [user], tools }, {
      maxRetries: 0, maxTokens: 4096,
      async onProviderStreamEvent(chunk) {
        const index = seen++;
        assert.deepEqual(chunk, chunks[index]);
        gates[index].reached.resolve();
        await gates[index].release.promise;
      },
    });
    const events = [];
    let terminal;
    const collect = (async () => {
      for await (const event of call) {
        assert.notEqual(event.type, "error", event.error?.errorMessage);
        events.push(eventView(event));
        if (event.type === "done") terminal = event.message;
      }
    })();
    const checkpoints = [];
    for (let index = 0; index < gates.length; index++) {
      await until(gates[index].reached.promise, "raw frame " + index);
      gates[index].release.resolve();
      // The next frame is held before normalization. A full turn also
      // lets the consumer read every already emitted event at this gate.
      if (gates[index + 1]) {
        await until(gates[index + 1].reached.promise, "next raw frame");
        await new Promise(setImmediate);
      } else await until(collect, "event EOF");
      checkpoints.push(events.length);
    }
    assert.equal(seen, chunks.length);
    const result = await call.result();
    assert.deepEqual(result, terminal);
    const messages = [user, result, toolResult];
    const replay = await until(models.streamSimple(gpt, { messages, tools }, { maxRetries: 0, maxTokens: 4096 }).result(), "Chat replay");
    assert.equal(replay.stopReason, "toolUse", replay.errorMessage);
    assert.equal(requests.length, 2);

    // Capture Anthropic's real body compiler through the public payload
    // callback and stop before dispatching a request.
    const claude = model("claude-sonnet-4-5-20250929", "anthropic-messages", "anthropic", base);
    let anthropicReplay;
    await until(anthropic.stream(claude, { messages, tools }, {
      apiKey: "fixture-key", cacheRetention: "none", maxTokens: 4096, maxRetries: 0,
      onPayload(payload) {
        anthropicReplay = payload.messages.find((message) => message.role === "assistant").content;
        throw new Error("payload captured");
      },
    }).result(), "Anthropic payload");
    assert.ok(anthropicReplay);
    assert.equal(requests.length, 2);
    return { events, checkpoints, result: view(result), chat_requests: requests, anthropic_replay: anthropicReplay };
  } finally {
    server.closeAllConnections();
    await new Promise((closed) => server.close(closed));
  }
}

async function durable(trace) {
  const [
    { Harness, LiveDoc, MemoryStorage, ToolResultEntry, defineTool },
    { nativeTimer },
    { allEntries, chatSetup, textOf },
    { addTool },
    { context, documentValues, followCommits },
    { obj, str, int },
  ] = await Promise.all([
    from("packages/durable/src/index.ts"),
    from("packages/durable/src/timers.ts"),
    from("packages/durable/test/chat-support.ts"),
    from("packages/durable/test/harness-support.ts"),
    from("packages/durable/test/session-support.ts"),
    from("packages/schema/src/index.ts"),
  ]);
  const setup = chatSetup();
  const progress = [];
  const executed = [];
  addTool(setup.registry, defineTool({
    name: "record", description: "Records fixture arguments", parameters: obj({ a: str(), n: int() }),
    async execute(args) {
      executed.push(structuredClone(args));
      return { output: [{ type: "text", text: "ok" }] };
    },
  }));

  let timer = Promise.withResolvers();
  const interval = 731;
  const reported = Promise.withResolvers();
  const held = ["toolcall_delta", "text_delta", "toolcall_delta"].map(() => ({
    reached: Promise.withResolvers(), release: Promise.withResolvers(), committed: Promise.withResolvers(),
  }));
  const gatedOffsets = [trace.checkpoints[2], trace.checkpoints[3], trace.checkpoints[4]].map((count) => count - 1);
  let nextCommit;
  let dispatches = 0;
  let replayed;
  function message(row) {
    const blocks = row.content.map(({ raw, ...block }) => block);
    const stopReason = row.stop === "tool_use" ? "toolUse" : row.stop;
    return fauxAssistantMessage(blocks, { stopReason, timestamp: 1 });
  }
  setup.faux.setResponses([(ctx) => {
    replayed = ctx.messages.find((entry) => entry.role === "assistant")?.content;
    return fauxAssistantMessage([fauxText("settled")]);
  }]);
  const models = new Proxy(setup.models, {
    get(target, property) {
      if (property === "streamSimple") return (model, ctx, options) => {
        if (++dispatches > 1) return target.streamSimple(model, ctx, options);
        const final = message(trace.result);
        return {
          async *[Symbol.asyncIterator]() {
            for (const [offset, row] of trace.events.entries()) {
              const partial = message(row.message);
              if (row.kind === "done") {
                yield { type: "done", reason: final.stopReason, message: final };
                continue;
              }
              yield {
                type: row.kind, contentIndex: row.index, delta: row.delta, content: row.content,
                toolCall: row.kind === "toolcall_end" ? partial.content[row.index] : undefined, partial,
              };
              const gate = gatedOffsets.indexOf(offset);
              if (gate !== -1) {
                held[gate].reached.resolve(row.kind);
                await held[gate].release.promise;
              }
            }
          },
          result: async () => final,
        };
      };
      const value = Reflect.get(target, property, target);
      return typeof value === "function" ? value.bind(target) : value;
    },
  });
  const harness = await Harness.open(new MemoryStorage(), {
    models, registry: setup.registry, settings: { progress: { partialIntervalMs: interval } },
    onReport: ({ error }) => reported.resolve(error),
    setTimer(callback, delay, options) {
      if (delay !== interval) return nativeTimer(callback, delay, options);
      timer.resolve(callback);
      return () => {};
    },
  }, context);
  try {
    const root = await harness.root(context, { agent: { model: { provider: "faux", modelId: "faux-1" } } });
    followCommits(harness, (publication) => {
      for (const change of documentValues(publication)) {
        const partial = change.value?.generation?.message;
        if (change.record.kind === LiveDoc.definition.kind && partial && nextCommit) {
          nextCommit.resolve(structuredClone(partial.content));
        }
      }
    });
    harness.resume();
    const submission = await root.submit({ type: "input", content: "fixture" }, context);
    const outcome = submission.wait(context);
    for (const [index, gate] of held.entries()) {
      const checkpoint = await until(Promise.race([
        gate.reached.promise,
        outcome.then((result) => assert.fail("settled before progress " + result.status)),
        reported.promise.then((error) => { throw error; }),
      ]), "Durable event");
      assert.equal(checkpoint, ["toolcall_delta", "text_delta", "toolcall_delta"][index]);
      nextCommit = gate.committed;
      const flush = await until(timer.promise, "Durable progress timer");
      timer = Promise.withResolvers();
      flush();
      progress.push(await until(gate.committed.promise, "Durable progress publication"));
      nextCommit = undefined;
      gate.release.resolve();
    }
    const result = await until(outcome, "Durable tool and settlement");
    const entries = await allEntries(root);
    const results = entries.filter((entry) => ToolResultEntry.is(entry));
    assert.equal(result.status, "done");
    assert.equal(results.length, 1);
    const answers = entries.flatMap((entry) => entry.model ?? []).filter((entry) => entry.role === "assistant");
    assert.equal(textOf(answers.at(-1)), "settled");
    assert.deepEqual(answers[0].content, replayed);
    return {
      progress, executed, dispatches, settlement: result.status, tool_result: textOf(results[0].model[0]),
      stored: answers[0].content, replayed, answer: textOf(answers.at(-1)),
    };
  } finally {
    for (const gate of held) gate.release.resolve();
    await harness.close(context);
  }
}

const fixtureBody = await readFile(fixture, "utf8");
const chatTrace = {
  resumed_text: await observe(fixtureBody),
  resumed_whitespace: await observe(fixtureBody.replace("😀EF", " \\n")),
};
let consumers;
if (beforeTrace) {
  const [before, after] = await Promise.all(
    [beforeTrace, afterTrace].map(async (path) => JSON.parse(await readFile(path, "utf8"))),
  );
  consumers = {
    pi: await durable(chatTrace.resumed_text),
    opi_before: await durable(before.resumed_text),
    opi_after: await durable(after.resumed_text),
  };
}
console.log(JSON.stringify({ chat: chatTrace, durable: consumers }));
