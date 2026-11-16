import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_deferred_failure_consumer.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { fauxAssistantMessage },
  { isContextOverflow },
  { isRetryableAssistantError },
  { MemoryStorage },
  { allEntries, chatSetup, openChat, textOf },
  { context },
] = await Promise.all([
  fromSnapshot("packages/ai/src/index.ts"),
  fromSnapshot("packages/ai/src/utils/overflow.ts"),
  fromSnapshot("packages/ai/src/utils/retry.ts"),
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

async function observe({ name, stop, diagnostic, decision }) {
  const setup = chatSetup({
    models: [{ id: "faux-1", contextWindow: 100_000, maxTokens: 900 }],
    deferred: { pollAfterMs: 1 },
  });
  const compacts = decision === "compact";
  const localAbort = decision === "abort";
  setup.settings.stream = { cacheRetention: "none" };
  setup.settings.retry = { enabled: true, maxRetries: 1, baseDelayMs: 1 };
  setup.settings.compaction = { enabled: false, reserveTokens: 1000, keepRecentTokens: 150, backgroundTokens: 0 };
  const { harness, root } = await openChat(new MemoryStorage(), setup);
  const fetchStarted = Promise.withResolvers();
  const release = Promise.withResolvers();
  try {
    harness.resume();
    if (compacts) {
      setup.faux.setResponses(Array.from({ length: 3 }, () => fauxAssistantMessage("answer ".repeat(120))));
      for (let index = 0; index < 3; index++) {
        const turn = await root.submit({ type: "input", content: "history ".repeat(120) }, context);
        assert.equal((await within(turn.wait(context), "history")).status, "done");
      }
      setup.settings.compaction = { ...setup.settings.compaction, enabled: true };
    }
    const priorEntries = (await allEntries(root)).length;
    setup.settings.stream = { cacheRetention: "none", deferred: true };
    const base = setup.faux.provider;
    const submits = [];
    const fetches = [];
    setup.models.setProvider({
      ...base,
      streamSimple(model, request, options) {
        submits.push({ deferred: options?.deferred === true });
        return base.streamSimple(model, request, options);
      },
      fetchDeferred(model, handle, options) {
        const signal = options?.signal;
        assert.ok(signal);
        if (localAbort) {
          if (signal.aborted) release.resolve();
          else signal.addEventListener("abort", () => release.resolve(), { once: true });
        }
        const source = base.fetchDeferred(model, handle, options);
        fetches.push({
          handle: structuredClone(handle), wait: options?.wait ?? null,
          terminal: source.result().then((message) => ({
            stop: message.stopReason,
            diagnostic: message.errorMessage ?? null,
            partial: textOf(message) ?? null,
            retryableMessage: isRetryableAssistantError(message),
            contextOverflow: isContextOverflow(message),
            signalAborted: signal.aborted,
          })),
        });
        return source;
      },
    });

    const replies = [fauxAssistantMessage("partial", { stopReason: stop, errorMessage: diagnostic })];
    if (decision === "retry" || compacts) {
      if (compacts) replies.push(fauxAssistantMessage("summary"));
      replies.push(fauxAssistantMessage("recovered"));
    }
    setup.faux.setResponses(localAbort ? [async () => {
      fetchStarted.resolve();
      await release.promise;
      return fauxAssistantMessage("held response");
    }] : replies);

    const input = await root.submit({ type: "input", content: "question ".repeat(120) }, context);
    if (localAbort) {
      await within(fetchStarted.promise, "in-flight provider fetch");
      await within(root.abort(context), "application abort");
    }
    const outcome = await within(input.wait(context), name + " input settlement");
    const entries = (await allEntries(root)).slice(priorEntries);
    const assistant = entries.filter((entry) => entry.kind === "pi.assistant").map((entry) => {
      const message = entry.model[0];
      return { stop: message.stopReason, diagnostic: message.errorMessage ?? null, text: textOf(message) ?? null };
    });
    const compactions = entries.filter((entry) => entry.kind === "pi.compaction").map((entry) => entry.data.reason);
    const terminals = await within(Promise.all(fetches.map((fetch) => fetch.terminal)), "provider terminals");
    const cancellations = setup.faux.state.cancelledDeferred;
    const distinctJobs = new Set(fetches.map((fetch) => fetch.handle.id)).size;
    const appRecovered = decision === "retry" || compacts;

    assert.equal(fetches.length, appRecovered ? 2 : 1);
    assert.equal(distinctJobs, fetches.length, "an application retry submits a new response");
    assert.ok(fetches.every((fetch) => fetch.wait === null));
    assert.deepEqual(submits.map((submit) => submit.deferred), compacts ? [true, false, true] : appRecovered ? [true, true] : [true]);
    assert.deepEqual(compactions, compacts ? ["overflow"] : []);
    assert.equal(cancellations.length, localAbort ? 1 : 0);
    assert.equal(terminals[0].signalAborted, localAbort);
    if (localAbort) {
      assert.equal(terminals[0].stop, "aborted");
      assert.deepEqual(cancellations[0], fetches[0].handle);
      assert.equal(outcome.status, "unanswered");
      assert.equal(outcome.reason, "aborted");
      assert.deepEqual(assistant, []);
    } else {
      assert.deepEqual(assistant[0], { stop, diagnostic, text: "partial" });
      assert.equal(terminals[0].retryableMessage, decision === "retry");
      assert.equal(terminals[0].contextOverflow, compacts);
      if (appRecovered) {
        assert.equal(outcome.status, "done");
        assert.equal(assistant.at(-1).text, "recovered");
      } else {
        assert.equal(outcome.status, "unanswered");
        assert.equal(outcome.reason, "model_error");
        assert.equal(outcome.detail, diagnostic);
      }
    }
    assert.deepEqual(setup.reports, []);
    return {
      name, outcome: { status: outcome.status, reason: outcome.reason ?? null, detail: outcome.detail ?? null },
      assistant, compactions, submits, terminals, distinctJobs,
      providerWait: fetches.map((fetch) => fetch.wait), remoteCancels: cancellations.length,
    };
  } finally {
    release.resolve();
    await within(harness.close(context), "harness close");
  }
}

const cases = [
  { name: "transient", stop: "error", diagnostic: "503 Service Unavailable", decision: "retry" },
  { name: "quota", stop: "error", diagnostic: "429 GoUsageLimitError: Monthly usage limit reached", decision: "fail" },
  { name: "context-window", stop: "error", diagnostic: "Context Window exceeded for given call.", decision: "compact" },
  { name: "request-size", stop: "error", diagnostic: "github-copilot API error (413): failed to parse request", decision: "compact" },
  { name: "provider-abort", stop: "aborted", diagnostic: "503 Service Unavailable", decision: "fail" },
  { name: "host-abort", stop: "aborted", diagnostic: "Request was aborted", decision: "abort" },
];
const observations = [];
for (const scenario of cases) observations.push(await observe(scenario));
console.log(JSON.stringify({ observations }));
