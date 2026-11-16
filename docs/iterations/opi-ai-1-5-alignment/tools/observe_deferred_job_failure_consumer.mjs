import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_deferred_job_failure_consumer.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { fauxAssistantMessage },
  { isContextOverflow },
  { isRetryableAssistantError },
  { MemoryStorage },
  { GenerationTask },
  { allEntries, chatSetup, openChat, waitFor },
  { addHooks },
  { context },
] = await Promise.all([
  fromSnapshot("packages/ai/src/index.ts"),
  fromSnapshot("packages/ai/src/utils/overflow.ts"),
  fromSnapshot("packages/ai/src/utils/retry.ts"),
  fromSnapshot("packages/durable/src/index.ts"),
  fromSnapshot("packages/durable/src/harness/generation.ts"),
  fromSnapshot("packages/durable/test/chat-support.ts"),
  fromSnapshot("packages/durable/test/harness-support.ts"),
  fromSnapshot("packages/durable/test/session-support.ts"),
]);

async function observe(name, diagnostic, retry = false) {
  const setup = chatSetup({ deferred: { pollAfterMs: 1 } });
  const retryPolicy = { enabled: true, maxRetries: 1, baseDelayMs: 1 };
  setup.settings.retry = retryPolicy;
  setup.settings.stream = { cacheRetention: "none", deferred: true };
  setup.settings.compaction = { enabled: false };
  setup.faux.setResponses([
    fauxAssistantMessage([], { stopReason: "error", errorMessage: diagnostic }),
    fauxAssistantMessage("recovered"),
  ]);
  const handled = [];
  let taskId;
  addHooks(setup.registry, GenerationTask, {
    afterResponse(message, runtime) {
      taskId ??= runtime.taskId;
      assert.equal(runtime.taskId, taskId);
      handled.push({
        stop: message.stopReason, diagnostic: message.errorMessage ?? null,
        retryable: isRetryableAssistantError(message), overflow: isContextOverflow(message),
        signalAborted: runtime.signal.aborted,
      });
    },
  });

  const { harness, root } = await openChat(new MemoryStorage(), setup);
  try {
    harness.resume();
    const input = await root.submit({ type: "input", content: "question" }, context);
    const outcome = await input.wait(context);
    const inputOutcome = { status: outcome.status, reason: outcome.reason ?? null, detail: outcome.detail ?? null };
    assert.ok(taskId);
    let task;
    await waitFor(async () => {
      task = (await harness.getTask(taskId, context))?.state;
      return task?.status === "terminal";
    });
    const assistant = (await allEntries(root)).filter((entry) => entry.kind === "pi.assistant").map((entry) => {
      const message = entry.model[0];
      return { stop: message.stopReason, diagnostic: message.errorMessage ?? null, contentBlocks: message.content.length };
    });
    const { callCount: submissions, deferredFetchCount: fetches, cancelledDeferred } = setup.faux.state;
    assert.equal(submissions, retry ? 2 : 1);
    assert.equal(fetches, submissions);
    assert.equal(handled.length, fetches);
    assert.deepEqual(cancelledDeferred, []);
    assert.deepEqual(handled[0], { stop: "error", diagnostic, retryable: retry, overflow: false, signalAborted: false });
    assert.deepEqual(assistant, [
      { stop: "error", diagnostic, contentBlocks: 0 },
      ...(retry ? [{ stop: "stop", diagnostic: null, contentBlocks: 1 }] : []),
    ]);
    assert.deepEqual(setup.reports, []);
    if (retry) {
      assert.equal(outcome.status, "done");
      assert.equal(task.outcome.status, "completed");
    } else {
      assert.deepEqual(inputOutcome, { status: "unanswered", reason: "model_error", detail: diagnostic });
      assert.deepEqual(task.outcome, {
        status: "failed", error: { message: diagnostic, detail: { reason: "model_error" } },
      });
    }
    return {
      name, submissions, fetches, remoteCancels: cancelledDeferred.length, handled, assistant,
      inputOutcome,
      taskOutcome: { status: task.outcome.status, error: task.outcome.error ?? null },
    };
  } finally {
    await harness.close(context);
  }
}

// Drive Durable with the exact diagnostics of the two normalized SDK traces.
// The paired lifecycle probes exercise the actual missing and cancelled job maps.
const observations = [];
for (const scenario of [
  ["pi-unknown-job", "Unknown faux deferred response: deferred-missing"],
  ["opi-unknown-job", "invalid provider configuration: Unknown scripted deferred response: deferred-missing"],
  ["pi-cancelled-job", "Faux deferred response was cancelled: deferred-1"],
  ["opi-cancelled-job", "invalid provider configuration: Scripted deferred response was cancelled: deferred-1"],
  ["transient-control", "503 Service Unavailable", true],
]) observations.push(await observe(...scenario));
console.log(JSON.stringify({ observations }));
