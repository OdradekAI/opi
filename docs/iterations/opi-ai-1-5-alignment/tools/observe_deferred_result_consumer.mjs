import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_deferred_result_consumer.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { fauxAssistantMessage },
  { AssistantEntry, MemoryStorage },
  { chatSetup, openChat, textOf },
  { context },
] = await Promise.all([
  fromSnapshot("packages/ai/src/index.ts"),
  fromSnapshot("packages/durable/src/index.ts"),
  fromSnapshot("packages/durable/test/chat-support.ts"),
  fromSnapshot("packages/durable/test/session-support.ts"),
]);

async function within(promise, checkpoint) {
  let deadline;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      deadline = setTimeout(() => reject(new Error("No progress at " + checkpoint)), 5000);
    })]);
  } finally {
    clearTimeout(deadline);
  }
}

const setup = chatSetup({ deferred: { pendingFetches: 1, pollAfterMs: 1 } });
setup.settings.stream = { deferred: true };
setup.faux.setResponses([fauxAssistantMessage("deferred ready")]);
const base = setup.faux.provider;
const gates = Array.from({ length: 2 }, () => ({
  reached: Promise.withResolvers(), release: Promise.withResolvers(),
  finished: Promise.withResolvers(), terminal: null, eof: false, finallyCalls: 0, resultReads: 0,
}));
const dispatches = [];

setup.models.setProvider({
  ...base,
  fetchDeferred(model, handle, options) {
    const gate = gates[dispatches.length];
    assert.ok(gate, "unexpected extra fetch");
    dispatches.push({ handle: structuredClone(handle), wait: options?.wait ?? null });
    const source = base.fetchDeferred(model, handle, options);
    return {
      async *[Symbol.asyncIterator]() {
        try {
          for await (const event of source) {
            yield event;
            if (event.type === "done") {
              gate.terminal = event.message.stopReason;
              gate.reached.resolve();
              await gate.release.promise;
            }
          }
          gate.eof = true;
        } finally {
          gate.finallyCalls++;
        }
      },
      async result() {
        gate.resultReads++;
        const result = await source.result();
        gate.finished.resolve();
        return result;
      },
    };
  },
});

const snapshotGates = () => gates.map(({ terminal, eof, finallyCalls, resultReads }) => ({
  terminal, eof, finallyCalls, resultReads,
}));
const { harness, root } = await openChat(new MemoryStorage(), setup);
try {
  harness.resume();
  const submission = await root.submit({ type: "input", content: "hi" }, context);
  await within(gates[0].reached.promise, "pending source tail");
  await within(gates[1].reached.promise, "ready source tail while the pending tail is held");
  const settled = await within(submission.wait(context), "answer commit while both tails are held");
  assert.equal(settled.status, "done");
  assert.equal(settled.type, "input");
  const answer = await root.commit((tx) => tx.entry(AssistantEntry, settled.answer), context);
  assert.equal(textOf(answer?.model?.[0]), "deferred ready");
  assert.deepEqual(dispatches[1], dispatches[0]);
  const beforeRelease = snapshotGates();
  assert.deepEqual(beforeRelease, ["deferred", "stop"].map((terminal) => ({
    terminal, eof: false, finallyCalls: 0, resultReads: 0,
  })));

  for (const gate of gates) gate.release.resolve();
  await within(Promise.all(gates.map((gate) => gate.finished.promise)), "source release");
  const afterRelease = snapshotGates();
  assert.deepEqual(afterRelease, ["deferred", "stop"].map((terminal) => ({
    terminal, eof: true, finallyCalls: 1, resultReads: 1,
  })));
  assert.equal(setup.faux.state.callCount, 1);
  assert.equal(setup.faux.state.deferredFetchCount, 2);
  assert.equal(setup.faux.state.cancelledDeferred.length, 0);
  console.log(JSON.stringify({
    beforeRelease, afterRelease, answer: textOf(answer?.model?.[0]),
    submissions: setup.faux.state.callCount, fetches: setup.faux.state.deferredFetchCount,
    sameHandle: true, providerWait: dispatches[0].wait, remoteCancels: setup.faux.state.cancelledDeferred.length,
  }));
} finally {
  for (const gate of gates) gate.release.resolve();
  await within(harness.close(context), "harness close");
}
