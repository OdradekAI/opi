import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createServer } from "node:http";
import { resolve } from "node:path";
import { setImmediate as turn, setTimeout as sleep } from "node:timers/promises";
import { pathToFileURL } from "node:url";

const [option, snapshot, fixture] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || !fixture || process.argv.length !== 5) {
  console.error("node observe_partial_aliases.mjs --snapshot <path> <sse-fixture>");
  process.exit(2);
}

const fromDist = (path) => import(pathToFileURL(resolve(snapshot, "packages/ai/dist", path)).href);
const [
  { createModels, createProvider },
  { stream, streamSimple },
  { fauxAssistantMessage, fauxProvider, fauxText, fauxToolCall },
] = await Promise.all([
  fromDist("models.js"),
  fromDist("api/openai-completions.js"),
  fromDist("providers/faux.js"),
]);

const sse = await readFile(fixture, "utf8");
const frames = sse.trimEnd().split(/\r?\n\r?\n/).map((row) => row.slice("data: ".length));
assert.equal(frames.pop(), "[DONE]");
const chunks = frames.map(JSON.parse);
const chatGroups = [
  ["start"], ["text_start", "text_delta"], ["text_delta"], ["toolcall_start", "toolcall_delta"],
  ["toolcall_delta"], ["toolcall_delta"], ["toolcall_delta"], ["toolcall_delta"], [], [],
  ["text_end", "toolcall_end", "done"],
];
assert.equal(chatGroups.length, chunks.length);
const fauxGroups = [
  ["text_delta"], ["text_delta", "text_end", "toolcall_start"],
  ["toolcall_delta"], ["toolcall_delta"], ["toolcall_delta"], ["toolcall_delta"],
  ["toolcall_delta", "toolcall_end", "done"],
];

function observe(call, mutate) {
  const iterator = call[Symbol.asyncIterator]();
  const events = [];
  const identities = new WeakMap();
  let nextIdentity = 0;
  function id(value) {
    if (!identities.has(value)) identities.set(value, ++nextIdentity);
    return identities.get(value);
  }
  const argumentsView = (value) => ({ ref: id(value), value: structuredClone(value) });
  const blockView = (block) => block.type === "text"
    ? { ref: id(block), kind: "text", text: block.text }
    : { ref: id(block), kind: "tool", id: block.id, name: block.name, args: argumentsView(block.arguments) };
  const view = (message) => ({
    ref: id(message), content_ref: id(message.content), content: message.content.map(blockView),
    stop: message.stopReason === "toolUse" ? "tool_use" : message.stopReason,
    response_id: message.responseId ?? null, input_tokens: message.usage.input,
  });
  let start, textStart, firstText, savedText, toolDelta, savedArgs, latest, done;
  const retained = () => ({
    start: view(start), text_start: view(textStart), first_text: view(firstText),
    saved_text: blockView(savedText), tool_delta: view(toolDelta), saved_args: argumentsView(savedArgs),
    latest: view(latest),
  });
  let write;
  let toolDeltas = 0;
  let pending;
  return {
    async expectPending() {
      pending ??= iterator.next();
      const waiting = await Promise.race([pending.then(() => false), turn(true)]);
      assert.equal(waiting, true, "unexpected public event at the metadata gate");
    },
    async read(types) {
      for (const type of types) {
        const item = await (pending ?? iterator.next());
        pending = undefined;
        assert.equal(item.done, false);
        const event = item.value;
        assert.equal(event.type, type, JSON.stringify(event));
        const message = event.partial ?? event.message;
        events.push({
          kind: type.replace("toolcall", "tool_call"),
          index: event.contentIndex ?? null,
          delta: event.delta ?? null,
          content: event.content ?? null,
          message: view(message),
        });
        if (type === "done") {
          done = message;
          continue;
        }
        latest = message;
        if (type === "start") start = message;
        if (type === "text_start") {
          textStart = message;
          savedText = message.content[event.contentIndex];
        }
        if (type === "text_delta" && !firstText) firstText = message;
        if (type === "toolcall_delta" && ++toolDeltas === 2) {
          toolDelta = message;
          savedArgs = message.content[event.contentIndex].arguments;
          if (mutate) {
            const before = retained();
            start.responseId = "consumer-id";
            savedText.text += "|consumer|";
            savedArgs.consumer = true;
            write = { before, after: retained() };
          }
        }
      }
    },
    retained,
    async finish(extra) {
      assert.equal((await iterator.next()).done, true);
      const result = await call.result();
      assert.strictEqual(result, done);
      return { events, write: write ?? null, after_terminal: retained(), result: view(result), ...extra };
    },
  };
}

async function openAi(mutate) {
  const requests = [];
  const server = createServer(async (request, response) => {
    let body = "";
    for await (const part of request) body += part;
    assert.equal(request.method, "POST");
    assert.equal(request.url, "/v1/chat/completions");
    assert.equal(request.headers.authorization, "Bearer fixture-key");
    const payload = JSON.parse(body);
    assert.equal(payload.model, "gpt-4o");
    assert.equal(payload.stream, true);
    assert.deepEqual(payload.messages, [{ role: "user", content: "fixture" }]);
    requests.push(payload);
    response.writeHead(200, { "content-type": "text/event-stream" });
    response.end(sse);
  });
  await new Promise((listen) => server.listen(0, "127.0.0.1", listen));
  try {
    const model = {
      id: "gpt-4o", name: "Fixture", api: "openai-completions", provider: "openai",
      baseUrl: "http://127.0.0.1:" + server.address().port + "/v1",
      reasoning: false, input: ["text"], contextWindow: 128000, maxTokens: 4096,
      cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    };
    const models = createModels();
    models.setProvider(createProvider({
      id: model.provider, models: [model], api: { stream, streamSimple },
      auth: { apiKey: { name: "Fixture", resolve: async () => ({ auth: { apiKey: "fixture-key" } }) } },
    }));
    const gates = chunks.map(() => ({ reached: Promise.withResolvers(), release: Promise.withResolvers() }));
    let rawCount = 0;
    const call = models.stream(model, { messages: [{ role: "user", content: "fixture", timestamp: 1 }] }, {
      maxRetries: 0, maxTokens: 4096,
      async onProviderStreamEvent(chunk) {
        const index = rawCount++;
        assert.deepEqual(chunk, chunks[index]);
        gates[index].reached.resolve();
        await gates[index].release.promise;
      },
    });
    const observation = observe(call, mutate);
    let afterUsage;
    for (const [index, group] of chatGroups.entries()) {
      await gates[index].reached.promise;
      gates[index].release.resolve();
      // The next frame is held before normalization. This also acknowledges
      // the metadata-only frame without requiring a public event from it.
      await gates[index + 1]?.reached.promise;
      await observation.read(group);
      if (chunks[index].usage) {
        await observation.expectPending();
        afterUsage = observation.retained();
      }
    }
    assert.equal(requests.length, 1);
    assert.equal(rawCount, chunks.length);
    return await observation.finish({
      requests: requests.length, request: requests[0], raw_frames: rawCount, after_usage: afterUsage,
    });
  } finally {
    server.closeAllConnections();
    await new Promise((closed) => server.close(closed));
  }
}

async function faux(mutate) {
  // Control Faux's chunk-delay timer while preserving its real provider and
  // event stream. No wall-clock race with the public iterator decides a read.
  const delay = 137;
  const timers = fauxGroups.map(() => Promise.withResolvers());
  let scheduled = 0;
  const original = globalThis.setTimeout;
  globalThis.setTimeout = (callback, ms, ...args) => {
    if (ms !== delay) return original(callback, ms, ...args);
    const gate = timers[scheduled++];
    assert.ok(gate, "unexpected Faux chunk");
    gate.resolve(() => callback(...args));
  };
  try {
    const models = createModels();
    const provider = fauxProvider({
      provider: "faux", api: "faux", tokenSize: { min: 1, max: 1 }, tokensPerSecond: 1000 / delay,
    });
    const script = fauxAssistantMessage([
      fauxText("ABCD😀EF"), fauxToolCall("record", { a: "BCDEF", n: 2 }, { id: "call_alias" }),
    ], { stopReason: "toolUse", responseId: "faux-alias", timestamp: 1 });
    models.setProvider(provider.provider);
    provider.setResponses([script]);
    const observation = observe(models.stream(provider.getModel(), { messages: [] }), mutate);
    await timers[0].promise;
    await observation.read(["start", "text_start"]);
    if (mutate) {
      script.content[0].text = "changed script";
      script.content[1].arguments.a = "changed script";
    }
    for (const [index, group] of fauxGroups.entries()) {
      (await timers[index].promise)();
      await timers[index + 1]?.promise;
      await observation.read(group);
    }
    assert.equal(scheduled, fauxGroups.length);
    assert.equal(provider.state.callCount, 1);
    return await observation.finish({
      requests: 1, chunks: scheduled, after_usage: null,
      source_after: script.content.map((block) => block.text ?? block.arguments),
    });
  } finally {
    globalThis.setTimeout = original;
  }
}

async function durable(owned) {
  const fromSource = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
  const [
    { AssistantEntry, Harness, LiveDoc, MemoryStorage },
    { nativeTimer },
    { chatSetup, textOf },
    { context, documentValues, followCommits },
  ] = await Promise.all([
    fromSource("packages/durable/src/index.ts"),
    fromSource("packages/durable/src/timers.ts"),
    fromSource("packages/durable/test/chat-support.ts"),
    fromSource("packages/durable/test/session-support.ts"),
  ]);
  const setup = chatSetup();
  const interval = 731;
  const timer = Promise.withResolvers();
  const usageReady = Promise.withResolvers();
  const release = Promise.withResolvers();
  const committed = Promise.withResolvers();
  const reported = Promise.withResolvers();
  const live = fauxAssistantMessage([fauxText("")], { stopReason: "pending", timestamp: 1 });
  live.usage = { ...live.usage };
  const snapshotAtEvent = () => owned ? structuredClone(live) : live;
  let calls = 0;
  const models = new Proxy(setup.models, {
    get(target, property) {
      if (property === "streamSimple") return () => {
        calls++;
        return {
          async *[Symbol.asyncIterator]() {
            yield { type: "start", partial: snapshotAtEvent() };
            yield { type: "text_start", contentIndex: 0, partial: snapshotAtEvent() };
            live.content[0].text = "ABCD";
            yield { type: "text_delta", contentIndex: 0, delta: "ABCD", partial: snapshotAtEvent() };
            live.usage = { ...live.usage, input: 7, totalTokens: 7 };
            usageReady.resolve();
            await release.promise;
            live.content[0].text += "😀EF";
            yield { type: "text_delta", contentIndex: 0, delta: "😀EF", partial: snapshotAtEvent() };
            yield { type: "text_end", contentIndex: 0, content: live.content[0].text, partial: snapshotAtEvent() };
            live.stopReason = "stop";
            yield { type: "done", reason: "stop", message: live };
          },
          result: async () => live,
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
    const root = await checkpoint(
      harness.root(context, { agent: { model: { provider: "faux", modelId: "faux-1" } } }),
      "Durable root",
    );
    followCommits(harness, (publication) => {
      for (const change of documentValues(publication)) {
        const message = change.value?.generation?.message;
        if (change.record.kind === LiveDoc.definition.kind && message) {
          committed.resolve({ text: textOf(message), input_tokens: message.usage.input });
        }
      }
    });
    harness.resume();
    const submission = await checkpoint(root.submit({ type: "input", content: "fixture" }, context), "Durable submit");
    await checkpoint(Promise.race([
      usageReady.promise,
      submission.wait(context).then((result) => assert.fail("generation settled before usage: " + JSON.stringify(result))),
    ]), "Durable usage frame");
    (await checkpoint(timer.promise, "Durable progress timer"))();
    const progress = await checkpoint(Promise.race([
      committed.promise, reported.promise.then((error) => { throw error; }),
    ]), "Durable progress publication");
    assert.deepEqual(progress, { text: "ABCD", input_tokens: owned ? 0 : 7 });
    release.resolve();
    const result = await checkpoint(submission.wait(context), "Durable settlement");
    assert.equal(result.status, "done");
    const answer = await root.commit((tx) => tx.entry(AssistantEntry, result.answer), context);
    const message = answer.model[0];
    assert.equal(textOf(message), "ABCD😀EF");
    assert.equal(message.usage.input, 7);
    assert.equal(calls, 1);
    return { progress, settlement: result.status, calls, final: { text: textOf(message), input_tokens: 7 } };
  } finally {
    release.resolve();
    await harness.close(context);
  }
}

async function checkpoint(promise, name) {
  const controller = new AbortController();
  try {
    return await Promise.race([
      promise,
      sleep(5000, undefined, { signal: controller.signal }).then(() => assert.fail("no progress at " + name)),
    ]);
  } finally {
    controller.abort();
  }
}

console.log(JSON.stringify({
  openai: { baseline: await checkpoint(openAi(false), "OpenAI"), mutated: await checkpoint(openAi(true), "OpenAI mutation") },
  faux: { baseline: await checkpoint(faux(false), "Faux"), mutated: await checkpoint(faux(true), "Faux mutation") },
  durable: { shared: await durable(false), owned: await durable(true) },
}));
