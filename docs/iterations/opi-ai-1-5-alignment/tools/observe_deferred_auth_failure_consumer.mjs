import assert from "node:assert/strict";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const [option, snapshot] = process.argv.slice(2);
if (option !== "--snapshot" || !snapshot || process.argv.length !== 4) {
  console.error("node --conditions=source observe_deferred_auth_failure_consumer.mjs --snapshot <path>");
  process.exit(2);
}
const fromSnapshot = (path) => import(pathToFileURL(resolve(snapshot, path)).href);
const [
  { createModels, fauxAssistantMessage, fauxProvider },
  { isContextOverflow },
  { isRetryableAssistantError },
  { Harness, MemoryStorage, createRegistry },
  { GenerationTask },
  { allEntries, waitFor },
  { addHooks },
  { context },
] = await Promise.all([
  fromSnapshot("packages/ai/dist/index.js"),
  fromSnapshot("packages/ai/dist/utils/overflow.js"),
  fromSnapshot("packages/ai/dist/utils/retry.js"),
  fromSnapshot("packages/durable/src/index.ts"),
  fromSnapshot("packages/durable/src/harness/generation.ts"),
  fromSnapshot("packages/durable/test/chat-support.ts"),
  fromSnapshot("packages/durable/test/harness-support.ts"),
  fromSnapshot("packages/durable/test/session-support.ts"),
]);

const provider = "lifecycle-differential";
const modelId = "mock-model";
const api = "scripted-deferred";
const retryPolicy = { enabled: true, maxRetries: 1, baseDelayMs: 1 };
const summarize = (message) => ({
  provider: message.provider, model: message.model, api: message.api, stop: message.stopReason,
  diagnostic: message.errorMessage ?? null, contentBlocks: message.content.length,
});

async function observe(projection, marker, retry) {
  const faux = fauxProvider({ provider, api, models: [{ id: modelId }], deferred: { pollAfterMs: 1 } });
  faux.setResponses([fauxAssistantMessage("first job"), fauxAssistantMessage("recovered")]);
  const models = createModels();
  const registry = createRegistry();
  const auth = { submit: 0, fetch: 0 };
  let fetching = false;
  models.setProvider({
    ...faux.provider,
    auth: { apiKey: { name: "Controlled resolver", resolve: async ({ signal }) => {
      signal.throwIfAborted();
      if (fetching) {
        if (++auth.fetch === 1) throw new Error(marker);
      } else {
        auth.submit++;
      }
      return { auth: {} };
    } } },
  });

  const piDiagnostic = `API key auth failed for provider ${provider}: ${marker}`;
  const diagnostic = projection === "opi" ? `authentication failed: ${marker}` : piDiagnostic;
  const polls = [];
  const jobs = new Set();
  const fetchDeferred = models.fetchDeferred.bind(models);
  models.fetchDeferred = async (model, handle, options) => {
    assert.equal(fetching, false);
    assert.deepEqual([model.provider, model.id, model.api], [provider, modelId, api]);
    assert.deepEqual([handle.provider, handle.modelId, handle.api], [provider, modelId, api]);
    assert.equal(options.signal.aborted, false);
    assert.equal(options.wait, undefined);
    jobs.add(handle.id);
    fetching = true;
    try {
      const dispatched = faux.state.deferredFetchCount;
      const message = await fetchDeferred(model, handle, options);
      const first = polls.length === 0;
      if (first) {
        assert.deepEqual(summarize(message), {
          provider, model: modelId, api, stop: "error", diagnostic: piDiagnostic, contentBlocks: 0,
        });
        assert.equal(message.deferred, undefined);
        assert.equal(message.usage.totalTokens, 0);
        assert.equal(message.usage.cost.total, 0);
      }
      polls.push({
        sdkDiagnostic: message.errorMessage ?? null,
        providerFetches: faux.state.deferredFetchCount - dispatched,
      });
      // Model the Rust caller handing CallFailure::message() to the application.
      return first && projection === "opi" ? { ...message, errorMessage: diagnostic } : message;
    } finally {
      fetching = false;
    }
  };

  let taskId;
  const handled = [];
  addHooks(registry, GenerationTask, {
    afterResponse(message, runtime) {
      taskId ??= runtime.taskId;
      assert.equal(runtime.taskId, taskId);
      handled.push({
        ...summarize(message), retryable: isRetryableAssistantError(message),
        overflow: isContextOverflow(message), signalAborted: runtime.signal.aborted,
      });
    },
  });
  const reports = [];
  const harness = await Harness.open(new MemoryStorage(), {
    models, registry, onReport: (report) => reports.push(report),
    settings: { retry: retryPolicy, stream: { cacheRetention: "none", deferred: true }, compaction: { enabled: false } },
  }, context);
  try {
    const root = await harness.root(context, { agent: { model: { provider, modelId } } });
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
    const assistant = (await allEntries(root)).filter((entry) => entry.kind === "pi.assistant")
      .map((entry) => summarize(entry.model[0]));
    const { callCount: submissions, cancelledDeferred } = faux.state;
    const attempts = retry ? 2 : 1;
    const failed = { provider, model: modelId, api, stop: "error", diagnostic, contentBlocks: 0 };
    assert.equal(submissions, attempts);
    assert.equal(jobs.size, attempts);
    assert.deepEqual(auth, { submit: attempts, fetch: attempts });
    assert.deepEqual(polls, [
      { sdkDiagnostic: piDiagnostic, providerFetches: 0 },
      ...(retry ? [{ sdkDiagnostic: null, providerFetches: 1 }] : []),
    ]);
    assert.deepEqual(cancelledDeferred, []);
    assert.deepEqual(reports, []);
    assert.equal(handled.length, attempts);
    assert.deepEqual(handled[0], { ...failed, retryable: retry, overflow: false, signalAborted: false });
    assert.deepEqual(assistant, [
      failed,
      ...(retry ? [{ provider, model: modelId, api, stop: "stop", diagnostic: null, contentBlocks: 1 }] : []),
    ]);
    if (retry) {
      assert.equal(inputOutcome.status, "done");
      assert.equal(task.outcome.status, "completed");
    } else {
      assert.deepEqual(inputOutcome, { status: "unanswered", reason: "model_error", detail: diagnostic });
      assert.deepEqual(task.outcome, {
        status: "failed", error: { message: diagnostic, detail: { reason: "model_error" } },
      });
    }
    return {
      projection, marker, submissions, polledJobs: jobs.size, retries: submissions - 1, auth, polls,
      remoteCancels: cancelledDeferred.length, handled, assistant, inputOutcome,
      taskOutcome: { status: task.outcome.status, error: task.outcome.error ?? null },
    };
  } finally {
    await harness.close(context);
  }
}

const observations = [];
for (const projection of ["pi", "opi"]) {
  observations.push(await observe(projection, "controlled-auth-setup-failure", false));
  observations.push(await observe(projection, "503 Service Unavailable", true));
}
console.log(JSON.stringify({ retryPolicy, observations }));
